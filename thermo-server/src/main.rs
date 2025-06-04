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
mod data_format;
mod safe_mpsc;
mod serial_comm;
mod temp_sensors;

pub use data_format::Measurement;
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
}

fn main() {
    // Initialize the logger
    env_logger::init();
    // Parse command line arguments
    let args = Args::parse();
    log::info!("Arguments: {args:#?}");
    let serial = PathBuf::from(&args.serial);
    if !serial.exists() {
        log::error!("[COM] Fatal error: {} does not exist.",  &args.serial);
        return;
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
    let temp_hdls = args
        .thermo_paths
        .iter()
        .filter_map(|path| {
            let path = PathBuf::from(format!("/dev/i2c-{path}"));
            if path.exists() {
                let running = running.clone();
                let sink = data_tx.clone();
                Some(thread::spawn({
                    move || onewire_thread(path, running, args.leds, sink)
                }))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let ser_hdl = {
        let running = running.clone();
        thread::spawn(move || serial_comm::serial_thread(args.serial, running, data_rx))
    };
    while running.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_secs(1));
    }
    if let Err(e) = ser_hdl.join() {
        log::error!("[COM] Thread panicked: {e:#?}");
    } else {
        log::info!("[COM] Thread joined successfully.");
    }
    for temp_hdl in temp_hdls {
        if let Err(e) = temp_hdl.join() {
            log::error!("[TMP] Thread panicked with error: {e:#?}");
        } else {
            log::info!("[TMP] Thread joined successfully.");
        }
    }
}
