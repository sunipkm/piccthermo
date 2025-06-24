use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use clap::Parser;

// Local imports
mod cpu_sensors;
mod data_format;
mod humi_sensors;
mod safe_mpsc;
mod serial_comm;
mod temp_sensors;

pub use data_format::Measurement;
use humi_sensors::humidity_thread;
use temp_sensors::onewire_thread;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// I2C bus IDs for temperature sensors (e.g. 0,1,2 for /dev/i2c-0, /dev/i2c-1, /dev/i2c-2)
    #[arg(
        long,
        use_value_delimiter = true,
        value_delimiter = ',',
        required = true
    )]
    thermo_paths: Vec<u8>,
    /// I2C bus IDs for humidity sensors (e.g. 0,1,2 for /dev/i2c-0, /dev/i2c-1, /dev/i2c-2)
    #[arg(long, use_value_delimiter = true, value_delimiter = ',')]
    humidity_paths: Vec<u8>,
    /// Serial port for data sink
    #[arg(long, required = true)]
    serial: String,
    /// Enable LED control
    #[arg(long, default_value_t = false)]
    leds: bool,
    /// Exclusion filter
    #[arg(long, default_value_t = String::from(""))]
    exclude: String,
}

fn main() {
    // Initialize the logger
    env_logger::init();
    // Parse command line arguments
    let args = Args::parse();
    log::info!("Arguments: {args:#?}");
    let serial = PathBuf::from(&args.serial);
    if !serial.exists() {
        log::error!("[COM] Fatal error: {} does not exist.", &args.serial);
        return;
    }
    // Exclusion filter
    let mut exclude = Vec::new();
    if !args.exclude.is_empty() {
        for item in args.exclude.split(',') {
            let item = item.trim().split("0x").last().unwrap_or(item);
            if let Ok(num) = u32::from_str_radix(item, 16) {
                exclude.push(num);
            } else {
                log::warn!("[MAIN] Invalid exclusion filter item: {item}");
            }
        }
        log::info!("[MAIN] Exclusion filter: {exclude:#?}");
    } else {
        log::info!("[MAIN] No exclusion filter set.");
    }
    // Synchronizer
    let running = Arc::new(AtomicBool::new(true));
    // Handle Ctrl+C to stop the server gracefully
    {
        let running = running.clone();
        ctrlc::set_handler(move || {
            log::info!("Received Ctrl+C, stopping the server...");
            running.store(false, Ordering::Relaxed);
        })
        .expect("Error setting Ctrl-C handler");
    }
    // Channel
    let (data_tx, data_rx) = safe_mpsc::channel();
    // Spawn the serial communication thread
    let ser_hdl = {
        let running = running.clone();
        thread::spawn(move || serial_comm::serial_thread(args.serial, running, data_rx))
    };
    // Spawn the temperature sensor threads
    let mut temp_hdls = args
        .thermo_paths
        .iter()
        .filter_map(|path| {
            let path = PathBuf::from(format!("/dev/i2c-{path}"));
            if path.exists() {
                let running = running.clone();
                let sink = data_tx.clone();
                let exclude = exclude.clone();
                Some(thread::spawn({
                    move || onewire_thread(path, running, args.leds, sink, exclude)
                }))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    temp_hdls.push(thread::spawn({
        let running = running.clone();
        let sink = data_tx.clone();
        move || cpu_sensors::cputemp_thread(running, sink)
    }));
    // Spawn humidity sensor threads if needed
    let hum_hdls = args
        .humidity_paths
        .iter()
        .filter_map(|path| {
            let path = PathBuf::from(format!("/dev/i2c-{path}"));
            if path.exists() {
                let running = running.clone();
                let sink = data_tx.clone();
                Some(thread::spawn({
                    move || humidity_thread(path, running, sink)
                }))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    // Main thread: wait for threads to finish
    while running.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_secs(1));
    }
    // Join temp sensor threads
    for temp_hdl in temp_hdls {
        if let Err(e) = temp_hdl.join() {
            log::error!("[TMP] Thread panicked with error: {e:#?}");
        } else {
            log::info!("[TMP] Thread joined successfully.");
        }
    }
    // Join humidity sensor threads
    for humi_hdl in hum_hdls {
        if let Err(e) = humi_hdl.join() {
            log::error!("[HUM] Thread panicked with error: {e:#?}");
        } else {
            log::info!("[HUM] Thread joined successfully.");
        }
    }
    // Join the serial communication thread
    if let Err(e) = ser_hdl.join() {
        log::error!("[COM] Thread panicked: {e:#?}");
    } else {
        log::info!("[COM] Thread joined successfully.");
    }
}
