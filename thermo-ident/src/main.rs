use cursive::views;
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

    siv.add_layer(views::Dialog::text("Try pressing Ctrl-C!"));

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

struct TempSensors {
    controllers: Vec<(
        Ds2484<linux_embedded_hal::I2cdev, linux_embedded_hal::Delay>,
        Option<Ds28ea00Group<32>>,
    )>,
}

use glob::glob;
impl TempSensors {
    fn new() -> Self {
        let val = glob("/dev/i2c-*")
            .expect("Failed to find I2C devices")
            .filter_map(Result::ok)
            .filter_map(|path| {
                let delay = linux_embedded_hal::Delay;
                if let Ok(i2c) = linux_embedded_hal::I2cdev::new(&path) {
                    if let Ok(mut ds2484) = ds2484::Ds2484Builder::default().build(i2c, delay) {
                        let mut cfg = ds2484::DeviceConfiguration::default();
                        if let Err(e) = cfg.read(&mut ds2484) {
                            log::error!("[TMP] Failed to read device configuration: {e:?}",);
                            return None;
                        }
                        cfg.set_active_pullup(true);
                        if let Err(e) = cfg.write(&mut ds2484) {
                            log::error!("[TMP] Failed to write device configuration: {e:?}",);
                            return None;
                        }
                        // Set the port configuration
                        let mut port_cfg = ds2484::OneWireConfigurationBuilder::default()
                            .reset_pulse(440000, 44000)
                            .presence_detect_time(58000, 5500)
                            .write_zero_low_time(52000, 5000)
                            .write_zero_recovery_time(2750)
                            .weak_pullup_resistor(1000)
                            .build();
                        if let Err(e) = port_cfg.write(&mut ds2484) {
                            log::error!("[TMP] Failed to write port configuration: {e:?}",);
                            return None;
                        } else {
                            log::info!("[TMP] Port configuration written successfully");
                        }
                        Some((ds2484, None))
                    } else {
                        log::error!(
                            "[TMP] Failed to create DS2484 instance for path: {}",
                            path.display()
                        );
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        TempSensors { controllers: val }
    }
}
