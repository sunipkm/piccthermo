use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use ds28ea00::{Ds28ea00Group, ReadoutResolution};
use ds2484::{DeviceConfiguration, Ds2484Builder, Interact, OneWireConfigurationBuilder};
use linux_embedded_hal::{Delay, I2cdev};

use crate::{Measurement, safe_mpsc};

pub fn onewire_thread(
    path: PathBuf,
    running: Arc<AtomicBool>,
    leds: bool,
    sink: safe_mpsc::SafeSender<Measurement>,
    exclude: Vec<u32>,
    no_overdrive: bool,
    print: bool,
) {
    let lpath = path.to_string_lossy();
    'root: while running.load(Ordering::Relaxed) {
        log::info!("[TMP] {lpath}> Opening bus",);
        // Open the I2C bus
        let mut i2c = match I2cdev::new(&path) {
            Ok(i2c) => {
                log::info!("[TMP] {lpath}> Bus opened successfully",);
                i2c
            }
            Err(e) => {
                log::error!("[TMP] {lpath}> Failed to open bus: {e}",);
                thread::sleep(Duration::from_secs(1));
                continue 'root;
            }
        };
        let mut delay = Delay;
        let mut ds2484 = match Ds2484Builder::default().build(&mut i2c, &mut delay) {
            Ok(ds2484) => {
                log::info!("[TMP] {lpath}> DS2484 instance created successfully",);
                ds2484
            }
            Err(e) => {
                log::error!("[TMP] {lpath}> Failed to create DS2484 instance: {e:?}",);
                thread::sleep(Duration::from_secs(1));
                continue 'root;
            }
        };
        let mut cfg = DeviceConfiguration::default();
        if let Err(e) = cfg.read(&mut ds2484) {
            log::error!("[TMP] {lpath}> Failed to read device configuration: {e:?}",);
            thread::sleep(Duration::from_secs(1));
            continue 'root;
        }
        cfg.set_active_pullup(true);
        if let Err(e) = cfg.write(&mut ds2484) {
            log::error!("[TMP] {lpath}> Failed to write device configuration: {e:?}",);
            thread::sleep(Duration::from_secs(1));
            continue 'root;
        }
        let mut port_cfg = OneWireConfigurationBuilder::default()
            .reset_pulse(440000, 44000)
            .presence_detect_time(58000, 5500)
            .write_zero_low_time(52000, 5000)
            .write_zero_recovery_time(2750)
            .weak_pullup_resistor(1000)
            .build();
        if let Err(e) = port_cfg.write(&mut ds2484) {
            log::error!("[TMP] {lpath}> Failed to write port configuration: {e:?}",);
        } else {
            log::info!("[TMP] {lpath}> Port configuration written successfully",);
        }
        let mut delay = Delay;
        let mut temp_sensors = Ds28ea00Group::<16>::default()
            .with_resolution(ReadoutResolution::Resolution12bit)
            .with_t_low(-40)
            .with_t_high(50)
            .with_toggle_pio(leds);
        match temp_sensors.enumerate(&mut ds2484) {
            Ok(devices) => {
                log::info!("[TMP] {lpath}> Found {devices} devices",);
                devices
            }
            Err(e) => {
                log::error!("[TMP] {lpath}> Failed to enumerate devices: {e:?}",);
                thread::sleep(Duration::from_secs(1));
                continue 'root;
            }
        };
        let roms = temp_sensors
            .roms()
            .map(|x| format!("0x{}", (x & 0x00ffffff_ffffffff) >> 8))
            .collect::<Vec<_>>();
        let roms = roms.join(", ");
        log::info!("[TMP] {lpath}> Roms enumerated: {roms}",);
        if !no_overdrive {
            log::info!("[TMP] {lpath}> Enabling overdrive mode",);
            if let Err(e) = temp_sensors.enable_overdrive(&mut ds2484) {
                log::error!("[TMP] {lpath}> Failed to enable overdrive mode: {e:?}",);
            }
            // At this point, we SHOULD have overdrive mode enabled
            // Do a conversion to verify
            if let Err(e) = temp_sensors.trigger_temperature_conversion(&mut ds2484, &mut delay) {
                match e {
                    ds2484::OneWireError::NoDevicePresent => {
                        log::warn!(
                            "[TMP] {lpath}> No device present on the bus in overdrive mode, disabling overdrive",
                        );
                        if let Err(e) = temp_sensors.disable_overdrive(&mut ds2484) {
                            log::error!("[TMP] {lpath}> Failed to disable overdrive mode: {e:?}",);
                        } else {
                            log::info!("[TMP] {lpath}> Overdrive mode disabled successfully",);
                        }
                    }
                    _ => {
                        log::error!("[TMP] {lpath}> Failed to trigger temperature conversion: {e:?}",);
                    }
                }
            }
        }
        // Do a readout
        'readout: while running.load(Ordering::Relaxed) {
            // Timekeeping
            let start = Instant::now();
            // Trigger temperature conversion
            if let Err(e) = temp_sensors.trigger_temperature_conversion(&mut ds2484, &mut delay) {
                log::error!("[TMP] {lpath}> Failed to trigger temperature conversion: {e:?}",);
                thread::sleep(Duration::from_secs(1));
                continue 'root;
            }
            // Wait for the conversion to complete
            let readout = match temp_sensors.read_temperatures(&mut ds2484, false, true) {
                Ok(readout) => readout,
                Err(e) => {
                    log::error!("[TMP] {lpath}> Failed to read temperatures: {e:?}",);
                    thread::sleep(Duration::from_secs(1));
                    continue 'readout;
                }
            };
            // Send the readout data here
            let data =
                readout
                    .iter()
                    .filter_map(|(id, temp)| {
                        let id = crc32fast::hash(&((id & 0x00ffffff_ffffffff) >> 8).to_le_bytes()); // strip the CRC and the family code bytes, and convert to u32 by calculating the CRC32 hash of the serial number bytes
                        if exclude.contains(&id) {
                            log::warn!(
                                "[TMP] {lpath}> Excluding sensor with ID {id:08x} from readout",
                            );
                            None // skip excluded sensors
                        } else {
                            let temp = f32::from(*temp);
                            Some((id, temp))
                        }
                    })
                    .collect::<Vec<_>>();
            if print {
                let mut msg = String::new();
                for (id, temp) in &data {
                    msg.push_str(&format!("{id:08x}: {temp:.2} °C, "));
                }
                log::info!("[TMP] {lpath}> {msg}");
            }
            if let Err(e) = sink.send(Measurement::Temperature(data)) {
                log::error!("[TMP] {lpath}> Failed to send data: {e:?}",);
                continue 'readout; // probably the receiver has been dropped, meaning we are leaving
            }
            // wait so that there is 1 second interval between measurements
            let dur = start.elapsed();
            if dur.as_secs_f32() < 1.0 {
                thread::sleep(Duration::from_secs_f32(1.0 - dur.as_secs_f32()));
            }
        }
    }
    log::info!("[TMP] {lpath}> Exiting thread",);
}
