use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use hdc1010::{Hdc1010Builder, SlaveAddress as H10SlaveAddress, Trigger};
use linux_embedded_hal::{Delay, I2cdev};

use crate::{Measurement, safe_mpsc};

pub fn humidity_thread(
    path: PathBuf,
    running: Arc<AtomicBool>,
    sink: safe_mpsc::SafeSender<Measurement>,
) {
    let lpath = path.to_string_lossy();
    'root: while running.load(Ordering::Relaxed) {
        log::info!("[HUM] {lpath}> Opening bus");

        // Open the I2C busString
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
                        log::info!(
                            "[HUM] {lpath}> Device found at address {:02x}",
                            hdc.get_address()
                        );
                        if let Err(e) = hdc.reset(&mut i2c, &mut delay) {
                            log::error!(
                                "[HUM] {lpath}> Error resetting sensor {:02x}: {e:?}.",
                                hdc.get_address()
                            );
                            return None;
                        }
                        Some(hdc)
                    }
                    Err(e) => {
                        log::warn!(
                            "[HUM] {lpath}> Address {:02x} not found: {e:?}",
                            addr.into_bits()
                        );
                        None
                    }
                }
            })
            .collect::<Vec<_>>();
        log::info!("[HUM] {lpath}> {} devices found.", hdc10s.len());
        std::thread::sleep(Duration::from_secs(1));
        while running.load(Ordering::Relaxed) {
            let start = Instant::now();
            if let Some(delay) = hdc10s
                .iter_mut()
                .filter_map(|hdc| {
                    hdc.trigger(&mut i2c, Trigger::Humidity)
                        .map_err(|e| {
                            log::warn!(
                                "[HUM] {lpath} Sensor 0x{:02x}: Could not trigger: {e:?}",
                                hdc.get_address()
                            );
                            e
                        })
                        .ok()
                })
                .max()
            {
                std::thread::sleep(delay);
                let mes = hdc10s
                    .iter_mut()
                    .filter_map(|hdc| match hdc.read_humidity(&mut i2c) {
                        Ok(r) => {
                            log::info!(
                                "[HUM] {lpath}> Sensor 0x{:02x}: {}%",
                                hdc.get_address(),
                                r.percentage()
                            );
                            Some((hdc.get_address() as u32, r.percentage()))
                        }
                        Err(e) => {
                            log::error!(
                                "[HUM] {lpath}> Sensor 0x{:02x}: Error reading: {e:?}",
                                hdc.get_address()
                            );
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                log::info!(
                    "[HUM] {lpath}> Read {} sensors in {:.2} ms.",
                    hdc10s.len(),
                    start.elapsed().as_secs_f64() * 1000.0
                );
                if let Err(e) = sink.send(Measurement::Humidity(mes)) {
                    log::error!("[HUM] {lpath}> We are leaving {e:?}.");
                    continue 'root;
                }
            }
            if start.elapsed().as_secs() < 1 {
                std::thread::sleep(Duration::from_secs(1) - start.elapsed());
            }
        }
    }
    log::info!("[HUM] {lpath}> Exiting thread.")
}
