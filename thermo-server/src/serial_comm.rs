use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

use crate::{Measurement, safe_mpsc};

const BOOT_CONFIG: &str = "/boot/firmware/cmdline.txt";
const BOOTLOADER_MODE_CMD: &str = "tmu_bootloader";

pub fn serial_thread(
    path: String,
    running: Arc<AtomicBool>,
    source: safe_mpsc::SafeReceiver<Measurement>,
) {
    log::info!("[COM] Serial thread started");
    'root: while running.load(Ordering::Relaxed) {
        source.set_ready(false);
        let ser = serialport::new(&path, 115200).timeout(Duration::from_secs(1));
        let mut ser = match serialport::TTYPort::open(&ser) {
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
        let sig = Arc::new(AtomicBool::new(true));
        let reader = ser
            .try_clone_native()
            .expect("[COM] Failed to clone serial port for reading");
        let reader_hdl = {
            let sig = sig.clone();
            std::thread::spawn(move || serial_reader(reader, sig))
        };
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
                break 'readout;
            }
            if let Err(e) = ser.flush() {
                log::error!("[COM] Failed to flush serial port: {e}");
                break 'readout;
            }
        }
        log::info!("[COM] Closing serial port");
        sig.store(false, Ordering::Relaxed);
        reader_hdl.join().expect("[COM] Reader thread panicked");
    }
    log::info!("[COM] Serial thread exiting");
}

fn serial_reader(ser: serialport::TTYPort, running: Arc<AtomicBool>) {
    log::info!("[COM] Serial reader thread started");
    let mut ser = ser;
    let mut buf = [0u8; 256];
    while running.load(Ordering::Relaxed) {
        match ser.read(&mut buf) {
            Ok(n) => {
                let cmd = String::from_utf8_lossy(&buf[..n]);
                if !cmd.is_empty() {
                    log::info!("[COM] Received command: {cmd}");
                }
                if cmd.contains(BOOTLOADER_MODE_CMD) {
                    log::info!("[COM] Bootloader command received, exiting reader");
                    let path = PathBuf::from(BOOT_CONFIG);
                    if !path.exists() {
                        log::error!("[COM] Boot config file does not exist: {BOOT_CONFIG}");
                    } else {
                        log::info!("[COM] Reading boot config file: {BOOT_CONFIG}");
                        match fs::read_to_string(&path) {
                            Ok(content) => {
                                log::info!("[COM] Boot config content: {content}");
                                let content = content.replace("g_serial", "g_ether");
                                if let Err(e) = fs::write(&path, content) {
                                    log::error!("[COM] Failed to write boot config file: {e}");
                                } else {
                                    log::info!(
                                        "[COM] Boot config file updated successfully, rebooting system..."
                                    );
                                    if let Err(e) =
                                        std::process::Command::new("sudo").arg("reboot").status()
                                    {
                                        log::error!("[COM] Failed to reboot system: {e}");
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("[COM] Failed to read boot config file: {e}");
                            }
                        }
                    }
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut {
                    continue;
                } else {
                    log::error!("[COM] Error reading from serial port: {e}");
                    break;
                }
            }
        }
    }
    log::info!("[COM] Serial reader thread exiting");
}
