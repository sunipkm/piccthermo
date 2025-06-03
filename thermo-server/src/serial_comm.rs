use std::{sync::{
    atomic::{AtomicBool, Ordering}, mpsc, Arc
}, time::Duration};

use crate::{Measurement, safe_mpsc};

pub fn serial_thread(
    path: String,
    running: Arc<AtomicBool>,
    source: safe_mpsc::SafeReceiver<Measurement>,
) {
    log::info!("[Comm] Serial thread started");
    'root: while running.load(Ordering::Relaxed) {
        source.set_ready(false);
        let mut ser = match serialport::new(&path, 115200).open() {
            Ok(ser) => {
                log::info!("[Comm] Serial port opened successfully");
                ser
            }
            Err(e) => {
                log::error!("[Comm] Failed to open serial port: {e}");
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue 'root;
            }
        };
        if ser.set_timeout(std::time::Duration::from_secs(1)).is_err() {
            log::error!("[Comm] Failed to set serial port timeout");
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue 'root;
        }
        source.set_ready(true); // here we are ready to receive data from various streams
        log::info!("[Comm] Serial port is ready to receive data");
        'readout: while running.load(Ordering::Relaxed) {
            let samp = match source.receiver().recv_timeout(Duration::from_secs(2)) {
                Ok(samp) => samp,
                Err(e) => {
                    match e {
                        mpsc::RecvTimeoutError::Timeout => {
                            log::warn!("[Comm] Timeout while waiting for data: {e}");
                            continue 'readout;
                        }
                        mpsc::RecvTimeoutError::Disconnected => {
                            log::warn!("[Comm] Data source disconnected: {e}");
                            break 'root;
                        }
                    }
                }
            };
            if let Err(e) = ser.write_all(&samp.to_le_bytes()) {
                log::error!("[Comm] Failed to write data to serial port: {e}");
                continue 'root;
            }
        }
    }
    log::info!("[Comm] Serial thread stopped");
}
