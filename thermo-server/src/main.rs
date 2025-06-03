use std::{sync::{atomic::AtomicBool, Arc}, thread, time::Duration};

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
    /// Path to I2C bus (e.g., /dev/i2c-1)
    #[arg(
        short,
        long,
        use_value_delimiter = true,
        value_delimiter = ',',
        num_args = 1
    )]
    paths: Vec<u8>,
    #[arg(long)]
    serial: String,
    #[arg(long, default_value_t = false)]
    leds: bool,
}

fn main() {
    // Initialize the logger
    env_logger::init();
    // Parse command line arguments
    let args = Args::parse();
    log::info!("Arguments: {args:#?}");
    // Synchronizer
    let running = Arc::new(AtomicBool::new(true));
    // Handle Ctrl+C to stop the server gracefully
    {
        let running = running.clone();
        ctrlc::set_handler(move || {
            log::info!("Received Ctrl+C, stopping the server...");
            running.store(false, std::sync::atomic::Ordering::Relaxed);
        })
        .expect("Error setting Ctrl-C handler");
    }
    // Channel
    let (data_tx, data_rx) = safe_mpsc::channel();
    let _temp_hdls = args
        .paths
        .iter()
        .map(|path| {
            let running = running.clone();
            let sink = data_tx.clone();
            thread::spawn({
                let path = *path;
                move || onewire_thread(format!("/dev/i2c-{path}"), running, args.leds, sink)
            })
        })
        .collect::<Vec<_>>();
    let _ser_hdl = {
        let running = running.clone();
        thread::spawn(move || serial_comm::serial_thread(args.serial, running, data_rx))
    };
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
