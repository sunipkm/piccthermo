use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

use crate::{Measurement, safe_mpsc};

pub fn serial_thread(
    path: String,
    running: Arc<AtomicBool>,
    source: safe_mpsc::SafeReceiver<Measurement>,
) {
    log::info!("[COM] Serial thread started");
    'root: while running.load(Ordering::Relaxed) {
        source.set_ready(false);
        let mut ser = match serialport::new(&path, 115200).open() {
            Ok(ser) => {
                log::info!("[COM] Serial port opened successfully");
                ser
            }
            Err(e) => {
                log::error!("[COM] Failed to open serial port: {e}");
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue 'root;
            }
        };
        if ser.set_timeout(std::time::Duration::from_secs(1)).is_err() {
            log::error!("[COM] Failed to set serial port timeout");
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue 'root;
        }
        source.set_ready(true); // here we are ready to receive data from various streams
        log::info!("[COM] Serial sink is ready to receive data");
        'readout: while running.load(Ordering::Relaxed) {
            let samp = match source.receiver().recv_timeout(Duration::from_secs(2)) {
                Ok(samp) => samp,
                Err(e) => match e {
                    mpsc::RecvTimeoutError::Timeout => {
                        log::warn!("[COM] Timeout while waiting for data: {e}");
                        continue 'readout;
                    }
                    mpsc::RecvTimeoutError::Disconnected => {
                        log::warn!("[COM] Data source disconnected: {e}");
                        break 'root;
                    }
                },
            };
            if let Err(e) = ser.write_all(&samp.to_le_bytes()) {
                log::error!("[COM] Failed to write data to serial port: {e}");
                continue 'root;
            }
        }
    }
    log::info!("[COM] Serial thread exiting");
}
