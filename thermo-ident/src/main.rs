use cursive::{
    With,
    view::Resizable,
    views::{self, Dialog, ListView},
};
use ds28ea00::Ds28ea00Group;
use ds2484::{Ds2484, Interact};

fn main() {
    // Initialize the cursive logger.
    cursive::logger::init();

    // Create a new Cursive instance.
    let mut siv = cursive::default();

    // Clear the global callbacks for Ctrl-C to prevent the default behavior.
    siv.clear_global_callbacks(cursive::event::Event::CtrlChar('c'));

    // Set a custom callback for Ctrl-C to show quit confirmation dialog.
    siv.set_on_pre_event(cursive::event::Event::CtrlChar('c'), |s| {
        add_quit_layer(s);
    });

    // Set a custom callback for 'q' to show quit confirmation dialog.
    siv.set_on_pre_event(cursive::event::Event::Char('q'), |s| {
        add_quit_layer(s);
    });

    // Add a global callback for '~' to toggle the debug console.
    siv.add_global_callback('~', cursive::Cursive::toggle_debug_console);
    siv.add_global_callback('`', cursive::Cursive::toggle_debug_console);

    let sensors = TempSensors::new();
    let paths = sensors.paths.clone();
    siv.set_user_data(sensors);
    let list = ListView::new().with(|tree| {
        for (idx, path) in paths.iter().enumerate() {
            let path = path.clone();
            tree.add_child(
                format!("I2C Bus {}", idx + 1),
                views::LinearLayout::horizontal()
                    .child(
                        views::Button::new(path.clone(), move |s| {
                            log::info!("[TMP] Selected I2C Bus: {}", &path);
                            if let Some(subtree) = s.with_user_data(|sensors: &mut TempSensors| {
                                log::info!("[TMP] Selected I2C Bus: {}", &path);
                                ListView::new().with(|stree| {
                                    let sensor = &sensors.sensors[idx];
                                    let ndigits = sensor.roms().count().checked_ilog10().unwrap_or(0) as usize + 1;
                                    for (i, sensor) in sensor.roms().enumerate() {
                                        let sensor_id = sensor;
                                        let sensor_hash = crc32fast::hash(
                                            &((sensor_id & 0x00ffffff_ffffffff) >> 8).to_le_bytes(),
                                        );
                                        stree.add_child(
                                        format!(
                                            "[Sensor {:ndigits$}] 0x{:016x} 0x{:08x}",
                                            i + 1,
                                            sensor_id,
                                            sensor_hash,
                                        ),
                                        views::LinearLayout::horizontal()
                                            .child(views::Button::new("ON", move |s| {
                                                s.with_user_data(|sensors: &mut TempSensors| {
                                                sensors.toggle_led(idx, i, true);
                                                log::info!(
                                                    "[TMP] Toggled LED ON for sensor {} on bus {}",
                                                    i,
                                                    idx
                                                );
                                            });
                                            }).fixed_width(5))
                                            .child(views::Button::new("OFF", move |s| {
                                                s.with_user_data(|sensors: &mut TempSensors| {
                                                sensors.toggle_led(idx, i, false);
                                                log::info!(
                                                    "[TMP] Toggled LED OFF for sensor {} on bus {}",
                                                    i,
                                                    idx
                                                );
                                            });
                                            }).fixed_width(5)),
                                    );
                                    }
                                })
                            }) {
                                s.add_layer(
                                    Dialog::new()
                                        .title(format!("I2C Bus {}", idx + 1))
                                        .content(subtree)
                                        .button("All ON", move |s| {
                                            s.with_user_data(|sensors: &mut TempSensors| {
                                                sensors.toggle_led_all(idx, true);
                                                log::info!(
                                                    "[TMP] Toggled all LEDs ON for bus {}",
                                                    idx
                                                );
                                            });
                                        })
                                        .button("All OFF", move |s| {
                                            s.with_user_data(|sensors: &mut TempSensors| {
                                                sensors.toggle_led_all(idx, false);
                                                log::info!(
                                                    "[TMP] Toggled all LEDs OFF for bus {}",
                                                    idx
                                                );
                                            });
                                        })
                                        .button("Back", |s| {
                                            s.pop_layer();
                                        }),
                                );
                            }
                        })
                        .fixed_width(16),
                    )
                    .child(views::Button::new("Enumerate", move |s| {
                        s.with_user_data(|sensors: &mut TempSensors| {
                            if let Some(sensor) = sensors.sensors.get_mut(idx) {
                                if let Err(e) = sensor.enumerate(&mut sensors.buses[idx]) {
                                    log::error!(
                                        "[TMP] Failed to enumerate sensors on bus {}: {:?}",
                                        idx,
                                        e
                                    );
                                } else {
                                    log::info!(
                                        "[TMP] Successfully enumerated sensors on bus {}",
                                        idx
                                    );
                                }
                            } else {
                                log::warn!("[TMP] No sensors found for bus {}", idx);
                            }
                        });
                    })),
            );
        }
    });

    siv.add_layer(Dialog::new().title("I2C Buses").content(list));
    siv.run();
}

fn add_quit_layer(s: &mut cursive::Cursive) {
    s.add_layer(
        views::Dialog::text("Do you want to quit?")
            .button("Yes", |s| s.quit())
            .button("No", |s| {
                s.pop_layer();
            }),
    )
}

pub struct TempSensors {
    pub paths: Vec<String>,
    pub buses: Vec<Ds2484<linux_embedded_hal::I2cdev, linux_embedded_hal::Delay>>,
    pub sensors: Vec<ds28ea00::Ds28ea00Group<32>>,
}

use glob::glob;
impl TempSensors {
    fn new() -> Self {
        let mut paths = Vec::new();
        let mut buses = Vec::new();
        let mut sensors = Vec::new();

        for path in glob("/dev/i2c-*").expect("Failed to find I2C devices") {
            match path {
                Ok(path) => {
                    let lpath = path.to_string_lossy();
                    log::info!("[TMP] Found I2C device: {lpath}");
                    match linux_embedded_hal::I2cdev::new(&path) {
                        Err(e) => {
                            log::error!("[TMP] {lpath}> Failed to open I2C device: {e:?}");
                            continue;
                        }
                        Ok(i2c) => {
                            match ds2484::Ds2484Builder::default()
                                .build(i2c, linux_embedded_hal::Delay)
                            {
                                Err(e) => {
                                    log::error!(
                                        "[TMP] {lpath}> Failed to create DS2484 instance: {e:?}"
                                    );
                                    continue;
                                }
                                Ok(mut ds2484) => {
                                    log::info!(
                                        "[TMP] {lpath}> DS2484 instance created successfully"
                                    );
                                    let mut cfg = ds2484::DeviceConfiguration::default();
                                    if let Err(e) = cfg.read(&mut ds2484) {
                                        log::error!(
                                            "[TMP] {lpath}> Failed to read device configuration: {e:?}",
                                        );
                                        continue;
                                    }
                                    cfg.set_active_pullup(true);
                                    if let Err(e) = cfg.write(&mut ds2484) {
                                        log::error!(
                                            "[TMP] {lpath}> Failed to write device configuration: {e:?}",
                                        );
                                        continue;
                                    }
                                    // Set the port configuration
                                    let mut port_cfg =
                                        ds2484::OneWireConfigurationBuilder::default()
                                            .reset_pulse(440000, 44000)
                                            .presence_detect_time(58000, 5500)
                                            .write_zero_low_time(52000, 5000)
                                            .write_zero_recovery_time(2750)
                                            .weak_pullup_resistor(1000)
                                            .build();
                                    if let Err(e) = port_cfg.write(&mut ds2484) {
                                        log::error!(
                                            "[TMP] {lpath}> Failed to write port configuration: {e:?}",
                                        );
                                        continue;
                                    } else {
                                        log::info!(
                                            "[TMP] {lpath}> Port configuration written successfully"
                                        );
                                    }
                                    let mut tmpsensors =
                                        Ds28ea00Group::default().with_toggle_pio(false);
                                    match tmpsensors.enumerate(&mut ds2484) {
                                        Ok(n) => {
                                            log::info!("[TMP] {lpath}> Found {n} sensors");
                                        }
                                        Err(e) => {
                                            log::error!(
                                                "[TMP] {lpath}> Failed to enumerate sensors: {e:?}"
                                            );
                                        }
                                    }
                                    paths.push(lpath.to_string());
                                    buses.push(ds2484);
                                    sensors.push(tmpsensors);
                                }
                            }
                        }
                    }
                }
                Err(e) => log::error!("Failed to read glob pattern: {}", e),
            }
        }

        log::info!("[TMP] Found {} I2C devices", buses.len());
        TempSensors {
            paths,
            buses,
            sensors,
        }
    }

    pub fn toggle_led(&mut self, bus_idx: usize, sensor_idx: usize, enable: bool) {
        if let Some(bus) = self.buses.get_mut(bus_idx) {
            if let Some(sensor) = self.sensors.get_mut(bus_idx) {
                if let Some(rom) = sensor.roms().nth(sensor_idx) {
                    // Toggle the LED for the specified sensor
                    if let Err(e) = sensor.led_toggle(bus, rom, enable) {
                        log::error!(
                            "[TMP] Failed to toggle LED for sensor {}: {:?}",
                            sensor_idx,
                            e
                        );
                    } else {
                        log::info!(
                            "[TMP] Successfully toggled LED for sensor {} on bus {}",
                            sensor_idx,
                            bus_idx
                        );
                    }
                } else {
                    log::warn!(
                        "[TMP] No sensor found at index {} on bus {}",
                        sensor_idx,
                        bus_idx
                    );
                }
            } else {
                log::warn!("[TMP] No sensors found for bus {}", bus_idx);
            }
        } else {
            log::warn!("[TMP] No bus found at index {}", bus_idx);
        }
    }

    pub fn toggle_led_all(&mut self, bus_idx: usize, enable: bool) {
        if let Some(bus) = self.buses.get_mut(bus_idx) {
            if let Some(sensors) = self.sensors.get_mut(bus_idx) {
                if let Err(e) = sensors.led_toggle_all(bus, enable) {
                    log::error!(
                        "[TMP] Failed to toggle all LEDs on bus {}: {:?}",
                        bus_idx,
                        e
                    );
                } else {
                    log::info!("[TMP] Successfully toggled all LEDs on bus {}", bus_idx);
                }
            }
        }
    }
}
