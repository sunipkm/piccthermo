use std::f32;

use clap::Parser;
use ds28ea00::Ds28ea00Group;
use ds2484::{Ds2484, Interact};
use embedded_onewire::OneWireStatus;
use linux_embedded_hal::{Delay, I2cdev};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to I2C bus (e.g., /dev/i2c-1)
    #[arg(short, long)]
    path: String,
    /// Read temperatures from the sensors
    #[arg(long, default_value = "false")]
    read: bool,
    /// Exclusion filter
    #[arg(long, default_value_t = String::from(""))]
    exclude: String,
}

fn main() {
    // Initialize the logger
    env_logger::init();
    // Parse command line arguments
    let args = Args::parse();
    // Exclusion filter
    let mut exclude = Vec::new();
    if !args.exclude.is_empty() {
        for item in args.exclude.split(',') {
            let item = item.trim().split("0x").last().unwrap_or(item);
            if let Ok(num) = u32::from_str_radix(item, 16) {
                exclude.push(num);
            } else {
                log::warn!("[EXC] Invalid exclusion filter item: {item}");
            }
        }
        log::info!("[EXC] Exclusion filter: {exclude:#?}");
    } else {
        log::info!("[EXC] No exclusion filter set.");
    }
    init(args.path, args.read, exclude);
}

fn init(path: String, read: bool, exclude: Vec<u32>) {
    println!("Opening bus {path}");
    // Open the I2C bus
    let mut i2c = I2cdev::new(&path).expect("Failed to open I2C device");
    let mut delay = Delay;
    // Create a DS2484 instance
    let mut ds2484 = ds2484::Ds2484Builder::default()
        .build(&mut i2c, &mut delay)
        .expect("Failed to create DS2484 instance");
    let mut cfg = ds2484::DeviceConfiguration::default();
    cfg.read(&mut ds2484)
        .expect("Failed to read device configuration");
    cfg.set_active_pullup(true);
    cfg.write(&mut ds2484)
        .expect("Failed to write device configuration");
    // Set the port configuration
    let mut port_cfg = ds2484::OneWireConfigurationBuilder::default()
        .reset_pulse(440000, 44000)
        .presence_detect_time(58000, 5500)
        .write_zero_low_time(52000, 5000)
        .write_zero_recovery_time(2750)
        .weak_pullup_resistor(1000)
        .build();
    // Configure the DS2484 port
    port_cfg
        .write(&mut ds2484)
        .expect("Failed to write port configuration");
    // Read the current port configuration
    port_cfg
        .read(&mut ds2484)
        .expect("Failed to read port configuration");
    log::info!("Port configuration: {:?}", port_cfg);
    // Create a DS28EA00 temperature sensor group
    let mut temp_sensors = Ds28ea00Group::<16>::default()
        .with_resolution(ds28ea00::ReadoutResolution::Resolution12bit)
        .with_t_low(-40)
        .with_t_high(50)
        .with_toggle_pio(true);
    let mut delay = Delay;
    // Enumerate devices on the 1-Wire bus
    let devices = temp_sensors
        .enumerate(&mut ds2484)
        .expect("Failed to enumerate devices");
    log::info!("Found {} devices", devices);
    let roms = temp_sensors
        .roms()
        .map(|rom| {
            let romcode = (rom & 0x00ffffff_ffffffff) >> 8;
            let romhash = crc32fast::hash(&romcode.to_le_bytes());
            (rom, romhash)
        })
        .collect::<Vec<_>>();
    println!("Enumerated devices: ");
    for (rom, hash) in roms {
        println!(
            "\t0x{rom:016x} -> 0x{hash:08x} [Excluded: {}]",
            exclude.contains(&hash)
        );
    }
    if let Err(e) = temp_sensors.enable_overdrive(&mut ds2484) {
        println!("Failed to enable overdrive mode: {e:?}");
    };
    let mut status = ds2484::DeviceConfiguration::default();
    // Read the device configuration
    status
        .read(&mut ds2484)
        .expect("Failed to read device configuration");
    println!("Device configuration: {:?}", status);
    let mut status = ds2484::DeviceStatus::default();
    status
        .read(&mut ds2484)
        .expect("Failed to read device status");
    println!("Device status: {:?}", status);
    if !status.presence() {
        println!("No devices are present after enabling overdrive mode.");
    } else if read {
        for _ in 0..10 {
            read_sensors(
                &mut temp_sensors,
                &mut ds2484,
                &mut delay,
                exclude.as_slice(),
            )
            .expect("Failed to read sensors");
        }
    }
    println!("Disabling overdrive mode...");
    temp_sensors
        .disable_overdrive(&mut ds2484)
        .expect("Failed to disable overdrive mode");
    status
        .read(&mut ds2484)
        .expect("Failed to read device status");
    if !status.presence() {
        println!("No devices are present after disabling overdrive mode!");
    } else if read {
        for _ in 0..10 {
            read_sensors(
                &mut temp_sensors,
                &mut ds2484,
                &mut delay,
                exclude.as_slice(),
            )
            .expect("Failed to read sensors");
        }
    }
}

fn read_sensors(
    temp_sensors: &mut Ds28ea00Group<16>,
    ds2484: &mut Ds2484<&mut I2cdev, &mut Delay>,
    delay: &mut Delay,
    exclude: &[u32],
) -> Result<
    (),
    Box<dyn std::error::Error + Send + Sync>,
    // embedded_onewire::OneWireError<
    //     ds2484::Ds2484Error<<linux_embedded_hal::I2cdev as embedded_hal::i2c::ErrorType>::Error>,
    // >,
> {
    let start = std::time::Instant::now();
    temp_sensors
        .trigger_temperature_conversion(ds2484, delay)
        .expect("Failed to trigger temperature conversion");
    let after_conversion = std::time::Instant::now();
    // Read temperatures from the sensors
    let readout = temp_sensors
        .read_temperatures(ds2484, false, true)
        .expect("Failed to read temperatures");
    let after_reading = std::time::Instant::now();
    let output = readout
        .iter()
        .filter_map(|(rom, temp)| {
            let hash = crc32fast::hash(&((rom & 0x00ffffff_ffffffff) >> 8).to_le_bytes());
            if exclude.contains(&hash) {
                None
            } else {
                Some(format!(
                    "R{:02x}: {:.3}°C, ",
                    rom.to_be_bytes()[0],
                    f32::from(*temp)
                ))
            }
        })
        .collect::<Vec<_>>();
    let output = output.join(", ");
    println!(
        "Mode: {}, Temperatures: {}, Conversion time: {:#?}, Read time: {:#?}",
        {
            if temp_sensors.overdrive() {
                "Overdrive"
            } else {
                "Standard"
            }
        },
        output,
        after_conversion.duration_since(start),
        after_reading.duration_since(after_conversion)
    );
    Ok(())
}
