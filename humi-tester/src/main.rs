use std::time::{Duration, Instant};

use clap::Parser;
use hdc1010::{Hdc1010Builder, SlaveAddress as H10SlaveAddress, Trigger};
use linux_embedded_hal::{Delay, I2cdev};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to I2C bus (e.g., /dev/i2c-1)
    #[arg(short, long)]
    path: String,
}

fn main() {
    // Initialize the logger
    env_logger::init();
    // Parse command line arguments
    let args = Args::parse();
    init(args.path);
}

fn init(path: String) {
    println!("[HUM] Opening bus: {path}");
    // Open the I2C bus
    let mut i2c = I2cdev::new(&path).expect("Failed to open I2C device");
    let mut delay = Delay;
    // Open all available devices
    let addrs = [
        H10SlaveAddress::default(),
        H10SlaveAddress::default().with_a0(true),
        H10SlaveAddress::default().with_a1(true),
        H10SlaveAddress::default().with_a0(true).with_a1(true),
    ];
    let mut hdc10s = addrs
        .iter()
        .filter_map(|addr| {
            match Hdc1010Builder::default()
                .with_address(*addr)
                .build_mode_separate(&mut i2c)
            {
                Ok(mut hdc) => {
                    println!("[HUM] Device found at address {:02x}", hdc.get_address());
                    hdc.reset(&mut i2c, &mut delay).unwrap_or_else(|_| {
                        panic!("[HUM] Sensor 0x{:02x}: Could not reset.", hdc.get_address())
                    });
                    Some(hdc)
                }
                Err(e) => {
                    log::warn!("[HUM] Address {:02x} not found: {e:?}", addr.into_bits());
                    None
                }
            }
        })
        .collect::<Vec<_>>();

    println!("[HUM] Devices found: {}", hdc10s.len());
    std::thread::sleep(Duration::from_secs(1));

    loop {
        let start = Instant::now();
        if let Some(delay) = hdc10s
            .iter_mut()
            .filter_map(|hdc| {
                hdc.trigger(&mut i2c, Trigger::Humidity)
                    .map_err(|e| {
                        log::warn!(
                            "[HUM] Sensor 0x{:02x}: Could not trigger: {e:?}",
                            hdc.get_address()
                        );
                        e
                    })
                    .ok()
            })
            .max()
        {
            std::thread::sleep(delay);
            for hdc in hdc10s.iter_mut() {
                match hdc.read_humidity(&mut i2c) {
                    Ok(r) => log::info!(
                        "[HUM] Sensor 0x{:02x}: {}%",
                        hdc.get_address(),
                        r.percentage()
                    ),
                    Err(e) => log::warn!(
                        "[HUM] Sensor 0x{:02x}: Error reading: {e:?}",
                        hdc.get_address()
                    ),
                }
            }
            log::info!(
                "[HUM] Read {} sensors in {:.2} ms.",
                hdc10s.len(),
                start.elapsed().as_secs_f64() * 1000.0
            );
        }
        if start.elapsed().as_secs() < 1 {
            std::thread::sleep(Duration::from_secs(1) - start.elapsed());
        }
    }
}
