#![no_std]
#![deny(missing_docs)]
//! # DS28EA00
//!
//! A no-std implementation of the DS28EA00 1-Wire temperature sensors in a group.
use embedded_hal::delay::DelayNs;
use embedded_onewire::{
    OneWire, OneWireCrc, OneWireError, OneWireResult, OneWireSearch, OneWireSearchKind,
};
use fixed::types::I12F4;

#[derive(Debug)]
/// Represents a group of DS28EA00 devices on the 1-Wire bus.
/// This struct can handle up to `N` devices, where `N` is a compile-time constant.
pub struct Ds28ea00Group<const N: usize> {
    devices: usize,
    roms: [(u64, Temperature); N],
    resolution: ReadoutResolution,
    low: i8,
    high: i8,
    toggle_pio: bool,
    overdrive: bool,
}

impl<const N: usize> Default for Ds28ea00Group<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Ds28ea00Group<N> {
    #[inline]
    /// Returns the family code for the DS28EA00 devices.
    ///
    /// The family code is `0x42`, which is used to identify the DS28EA00 devices on the 1-Wire bus.
    pub const fn family() -> u8 {
        0x42
    }

    fn new() -> Self {
        Self {
            devices: 0,
            roms: [(0, Temperature::ZERO); N],
            resolution: ReadoutResolution::default(),
            low: -40,
            high: 85,
            toggle_pio: false,
            overdrive: false,
        }
    }

    /// Sets the temperature readout resolution for the DS28EA00 devices.
    pub fn with_resolution(mut self, resolution: ReadoutResolution) -> Self {
        self.resolution = resolution;
        self
    }

    /// Sets the temperature low threshold for the DS28EA00 devices.
    ///
    /// Devices at or below this temperature can be addressed with the [`ONEWIRE_CONDITIONAL_SEARCH_CMD`](embedded_onewire::ONEWIRE_CONDITIONAL_SEARCH_CMD).
    pub fn with_t_low(mut self, temp: i8) -> Self {
        self.low = temp;
        self
    }

    /// Sets the temperature high threshold for the DS28EA00 devices.
    ///
    /// Devices at or above this temperature can be addressed with the [`ONEWIRE_CONDITIONAL_SEARCH_CMD`](embedded_onewire::ONEWIRE_CONDITIONAL_SEARCH_CMD).
    pub fn with_t_high(mut self, temp: i8) -> Self {
        self.high = temp;
        self
    }

    /// Enables or disables the toggle PIO feature for the DS28EA00 devices.
    ///
    /// When enabled, the PIO pins of all devices are turned on while setting the configuration register,
    /// and then turned off after the configuration is applied.
    /// When reading temperatures, all PIO pins are turned on before starting the temperature conversion,
    /// and then turned off sequentially for each device after reading its temperature.
    pub fn with_toggle_pio(mut self, toggle_pio: bool) -> Self {
        self.toggle_pio = toggle_pio;
        self
    }

    /// Enumerates the DS28EA00 devices on the 1-Wire bus.
    ///
    /// This method searches for devices on the bus, addresses them, and applies the configuration settings.
    /// # Arguments
    /// * `bus` - A mutable reference to a type that implements the [`OneWire`] trait.
    ///
    /// # Returns
    /// A result containing the number of devices found and configured, or an error if the operation fails.
    pub fn enumerate<O: OneWire>(&mut self, bus: &mut O) -> OneWireResult<usize, O::BusError> {
        self.devices = 0; // reset device count
        let mut search = OneWireSearch::with_family(bus, OneWireSearchKind::Normal, Self::family());
        // conduct search
        while let Some(rom) = search.next()? {
            self.roms[self.devices].0 = rom;
            self.devices += 1;
            if self.devices == N {
                break;
            }
        }
        if self.toggle_pio {
            // turn all PIO pins on
            bus.address(None)?;
            bus.write_byte(DS28EA00_TOGGLE_PIO)?;
            bus.write_byte(DS28EA00_TOGGLE_PIO_OFF)?;
            bus.write_byte(DS28EA00_TOGGLE_PIO_ON)?;
        }
        // address all devices
        bus.address(None)?;
        // apply configuration
        bus.write_byte(DS28EA00_WRITE_SCRATCH)?;
        bus.write_byte(self.low as _)?;
        bus.write_byte(self.high as _)?;
        bus.write_byte(self.resolution as _)?;
        if self.toggle_pio {
            // turn all PIO pins off
            bus.address(None)?;
            bus.write_byte(DS28EA00_TOGGLE_PIO)?;
            bus.write_byte(DS28EA00_TOGGLE_PIO_ON)?;
            bus.write_byte(DS28EA00_TOGGLE_PIO_OFF)?;
        }
        Ok(self.devices)
    }

    /// Enumerate the ROMs found
    pub fn roms(&self) -> impl Iterator<Item = u64> {
        self.roms[..self.devices].iter().map(|(x, _)| *x)
    }

    /// Check if overdrive mode is enabled.
    pub fn overdrive(&self) -> bool {
        self.overdrive
    }

    /// Enable overdrive mode
    pub fn enable_overdrive<O: OneWire>(&mut self, bus: &mut O) -> OneWireResult<(), O::BusError> {
        bus.set_overdrive_mode(true)?; // set overdrive mode
        self.overdrive = true; // enable overdrive mode
        Ok(())
    }

    /// Disable overdrive mode
    pub fn disable_overdrive<O: OneWire>(&mut self, bus: &mut O) -> OneWireResult<(), O::BusError> {
        bus.set_overdrive_mode(false)?; // disable overdrive mode
        self.overdrive = false; // disable overdrive mode
        Ok(())
    }

    /// Triggers a temperature conversion on all DS28EA00 devices in the group.
    /// This method addresses all devices, sends the command to start the conversion,
    /// and waits for the conversion to complete based on the configured resolution.
    ///
    /// # Arguments
    /// * `bus` - A mutable reference to a type that implements the [`OneWire`] trait.
    /// * `delay` - A mutable reference to a type that implements the [`DelayNs`] trait to wait for the conversion to complete.
    pub fn trigger_temperature_conversion<O: OneWire, D: DelayNs>(
        &self,
        bus: &mut O,
        delay: &mut D,
    ) -> OneWireResult<(), O::BusError> {
        bus.address(None)?; // address all devices
        bus.write_byte(DS28EA00_START_CONV)?; // start temperature conversion
        if self.toggle_pio {
            bus.address(None)?; // address all devices
            bus.write_byte(DS28EA00_TOGGLE_PIO)?;
            bus.write_byte(DS28EA00_TOGGLE_PIO_OFF)?; // turn on PIO
            bus.write_byte(DS28EA00_TOGGLE_PIO_ON)?; // turn on PIO
        }
        delay.delay_us(self.resolution.delay_us()); // wait till conversion is finished
        Ok(())
    }

    /// Reads the temperatures from all DS28EA00 devices in the group.
    /// This method addresses each device, reads the temperature data, and validates the CRC if requested.
    /// # Arguments
    /// * `bus` - A mutable reference to a type that implements the [`OneWire`] trait.
    /// * `crc` - A boolean indicating whether to validate the CRC of the read data.
    /// # Returns
    /// A result containing a slice of tuples, each containing the ROM address and the temperature reading,
    /// or an error if the operation fails.
    pub fn read_temperatures<O: OneWire>(
        &mut self,
        bus: &mut O,
        crc: bool,
        ignore_errors: bool,
    ) -> OneWireResult<&[(u64, Temperature)], O::BusError> {
        for (rom, temp) in self.roms[..self.devices].iter_mut() {
            let res = Self::read_temperature_internal(bus, *rom, temp, crc, self.toggle_pio);
            if let Err(e) = res {
                if !ignore_errors {
                    return Err(e);
                } else {
                    *temp = Temperature::from_num(-85); // Set to -85 on error
                }
            }
        }
        Ok(&self.roms[..self.devices])
    }

    /// Reads the temperature from a specific DS28EA00 device.
    /// This method addresses the device by its ROM address, reads the temperature data,
    /// and validates the CRC if requested.
    /// # Arguments
    /// * `bus` - A mutable reference to a type that implements the [`OneWire`]` trait.
    /// * `rom` - The ROM address of the DS28EA00 device to read.
    /// * `crc` - A boolean indicating whether to validate the CRC of the read data.
    /// # Returns
    /// A result containing the temperature reading, or an error if the operation fails.
    ///
    pub fn read_temperature<O: OneWire, D: DelayNs>(
        &self,
        bus: &mut O,
        delay: &mut D,
        rom: u64,
        crc: bool,
    ) -> OneWireResult<Temperature, O::BusError> {
        let mut temp = Temperature::ZERO; // Initialize temperature
        self.trigger_temperature_conversion(bus, delay)?; // Trigger temperature conversion
        Self::read_temperature_internal(bus, rom, &mut temp, crc, self.toggle_pio)?; // Read temperature
        Ok(temp)
    }

    fn read_temperature_internal<O: OneWire>(
        bus: &mut O,
        rom: u64,
        temp: &mut Temperature,
        crc: bool,
        toggle_pio: bool,
    ) -> OneWireResult<(), O::BusError> {
        bus.address(Some(rom))?; // address device
        bus.write_byte(DS28EA00_READ_SCRATCH)?; // Read scratchpad
        if !crc {
            let mut buf = [0; 2];
            for b in buf.iter_mut() {
                *b = bus.read_byte()?;
            }
            *temp = I12F4::from_le_bytes([buf[0] & ReadoutResolution::default().bitmask(), buf[1]]);
        } else {
            let mut buf = [0; 9];
            for b in buf.iter_mut() {
                *b = bus.read_byte()?;
            }
            if OneWireCrc::validate(&buf) {
                *temp =
                    I12F4::from_le_bytes([buf[0] & ReadoutResolution::default().bitmask(), buf[1]]);
            } else {
                return Err(OneWireError::InvalidCrc);
            }
        }
        if toggle_pio {
            bus.address(Some(rom))?; // address device
            bus.write_byte(DS28EA00_TOGGLE_PIO)?;
            bus.write_byte(DS28EA00_TOGGLE_PIO_ON)?;
            bus.write_byte(DS28EA00_TOGGLE_PIO_OFF)?;
        }
        Ok(())
    }

    /// Turn on the LED of a DS28EA00 device.
    ///
    /// # Arguments
    /// * `rom` - The ROM address of the DS28EA00 device.
    /// * `bus` - A mutable reference to a type that implements the [`OneWire`] trait.
    /// * `enable` - A boolean indicating whether to turn the LED on (`true`) or off (`false`).
    pub fn led_toggle<O: OneWire>(
        &self,
        bus: &mut O,
        rom: u64,
        enable: bool,
    ) -> OneWireResult<(), O::BusError> {
        bus.address(Some(rom))?;
        bus.write_byte(DS28EA00_TOGGLE_PIO)?;
        if !enable {
            bus.write_byte(DS28EA00_TOGGLE_PIO_ON)?;
            bus.write_byte(DS28EA00_TOGGLE_PIO_OFF)?;
        } else {
            bus.write_byte(DS28EA00_TOGGLE_PIO_OFF)?;
            bus.write_byte(DS28EA00_TOGGLE_PIO_ON)?;
        }
        Ok(())
    }

    /// Turn the LED of all DS28EA00 devices in the group on or off.
    ///
    /// # Arguments
    /// * `bus` - A mutable reference to a type that implements the [`OneWire`] trait.
    /// * `enable` - A boolean indicating whether to turn the LED on (`true`) or off (`false`).
    pub fn led_toggle_all<O: OneWire>(
        &self,
        bus: &mut O,
        enable: bool,
    ) -> OneWireResult<(), O::BusError> {
        bus.address(None)?;
        bus.write_byte(DS28EA00_TOGGLE_PIO)?;
        if !enable {
            bus.write_byte(DS28EA00_TOGGLE_PIO_ON)?;
            bus.write_byte(DS28EA00_TOGGLE_PIO_OFF)?;
        } else {
            bus.write_byte(DS28EA00_TOGGLE_PIO_OFF)?;
            bus.write_byte(DS28EA00_TOGGLE_PIO_ON)?;
        }
        Ok(())
    }
}

/// Temperature data type used by the DS28EA00 devices.
///
/// This type represents a temperature value with a fixed-point format of 12 bits for the integer part and 4 bits for the fractional part.
pub type Temperature = I12F4;

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
/// Represents the readout resolution of the DS28EA00 devices.
/// The resolution determines the time required for the temperature conversion and the precision of the temperature readings.
pub enum ReadoutResolution {
    /// 9-bit resolution, with a conversion time of 93.75 ms.
    Resolution9bit = 0x1f,
    /// 10-bit resolution, with a conversion time of 187.5 ms.
    Resolution10bit = 0x3f,
    /// 11-bit resolution, with a conversion time of 375 ms.
    Resolution11bit = 0x5f,
    /// 12-bit resolution, with a conversion time of 750 ms.
    Resolution12bit = 0x7f,
}

impl Default for ReadoutResolution {
    fn default() -> Self {
        Self::Resolution12bit
    }
}

impl ReadoutResolution {
    pub(crate) fn delay_us(&self) -> u32 {
        use ReadoutResolution::*;
        match self {
            Resolution9bit => 93750,
            Resolution10bit => 187500,
            Resolution11bit => 375000,
            Resolution12bit => 750000,
        }
    }

    #[inline]
    pub(crate) fn bitmask(&self) -> u8 {
        use ReadoutResolution::*;
        match self {
            Resolution9bit => 0xf8,
            Resolution10bit => 0xfc,
            Resolution11bit => 0xfe,
            Resolution12bit => 0xff,
        }
    }
}

impl TryFrom<u8> for ReadoutResolution {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use ReadoutResolution::*;
        match value {
            0x1f => Ok(Resolution9bit),
            0x3f => Ok(Resolution10bit),
            0x5f => Ok(Resolution11bit),
            0x7f => Ok(Resolution12bit),
            _ => Err("Invalid readout resolution"),
        }
    }
}

#[allow(unused)]
const DS28EA00_READ_SCRATCH: u8 = 0xbe;
const DS28EA00_WRITE_SCRATCH: u8 = 0x4e;
#[allow(unused)]
const DS28EA00_COPY_SCRATCH: u8 = 0x48;
const DS28EA00_START_CONV: u8 = 0x44;
#[allow(unused)]
const DS28EA00_READ_POWERMODE: u8 = 0xb4;
#[allow(unused)]
const DS28EA00_RECALL_EEPROM: u8 = 0xb8;
const DS28EA00_TOGGLE_PIO: u8 = 0xa5;
const DS28EA00_TOGGLE_PIO_ON: u8 = 0b11111101;
const DS28EA00_TOGGLE_PIO_OFF: u8 = !0b11111101;
