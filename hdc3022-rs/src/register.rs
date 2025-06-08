use bitfield_struct::bitfield;
use embedded_hal::i2c::{I2c, SevenBitAddress};

use crate::{Error, core::Hdc3022};

pub(crate) const HDC3022_MANUFACTURER_ID: u16 = 0x3000; // Texas Instruments
pub(crate) const HDC3022_DEVICE_ID: u16 = 0x1000; // HDC3022 Device ID

pub(crate) trait Hdc3022Register: Default {
    const ADDRESS: u8;
    const REGISTER_LEN: usize;

    fn read<T: I2c<SevenBitAddress>>(
        &mut self,
        hdc: &mut Hdc3022<T>,
    ) -> Result<(), Error<T::Error>>;
    fn write<T: I2c<SevenBitAddress>>(
        &mut self,
        _hdc: &mut Hdc3022<T>,
    ) -> Result<(), Error<T::Error>> {
        Err(Error::ReadOnly)
    }
}

#[derive(Debug, PartialEq)]
/// Trigger a measurement for either temperature or humidity.
pub enum Trigger {
    /// Trigger a measurement for both temperature and humidity.
    Both,
    /// Trigger a temperature measurement.
    Temperature,
    /// Trigger a humidity measurement.
    Humidity,
}

#[derive(Debug, Default)]
/// Represents a temperature measurement from the HDC3022 sensor.
pub struct Temperature {
    pub(crate) value: u16,
}

impl Temperature {
    /// Converts the raw temperature value to Celsius.
    pub fn celsius(&self) -> core::primitive::f32 {
        // Convert the raw value to Celsius
        (self.value as f32 * 165.0 / 65536.0) - 40.0
    }
}

impl Hdc3022Register for Temperature {
    const ADDRESS: u8 = 0x0;

    const REGISTER_LEN: usize = 2;

    fn read<T: I2c<SevenBitAddress>>(
        &mut self,
        hdc: &mut Hdc3022<T>,
    ) -> Result<(), Error<T::Error>> {
        let mut buffer = [0u8; Self::REGISTER_LEN];
        hdc.i2c
            .write_read(hdc.address, &[Self::ADDRESS], &mut buffer)?;
        self.value = u16::from_be_bytes(buffer);
        Ok(())
    }

    fn write<T: I2c<SevenBitAddress>>(
        &mut self,
        hdc: &mut Hdc3022<T>,
    ) -> Result<(), Error<T::Error>> {
        hdc.i2c.write(hdc.address, &[Self::ADDRESS])?;
        Ok(())
    }
}

#[derive(Debug, Default)]
/// Represents a humidity measurement from the HDC3022 sensor.
pub struct Humidity {
    pub(crate) value: u16,
}

impl Humidity {
    /// Converts the raw humidity value to percentage (0-100).
    pub fn percentage(&self) -> core::primitive::f32 {
        self.value as f32 * 100.0 / 65536.0
    }
}

impl Hdc3022Register for Humidity {
    const ADDRESS: u8 = 0x1;

    const REGISTER_LEN: usize = 2;

    fn read<T: I2c<SevenBitAddress>>(
        &mut self,
        hdc: &mut Hdc3022<T>,
    ) -> Result<(), Error<T::Error>> {
        let mut buffer = [0u8; Self::REGISTER_LEN];
        hdc.i2c
            .write_read(hdc.address, &[Self::ADDRESS], &mut buffer)?;
        self.value = u16::from_be_bytes(buffer);
        Ok(())
    }

    fn write<T: I2c<SevenBitAddress>>(
        &mut self,
        hdc: &mut Hdc3022<T>,
    ) -> Result<(), Error<T::Error>> {
        hdc.i2c.write(hdc.address, &[Self::ADDRESS])?;
        Ok(())
    }
}

#[bitfield(u16)]
pub struct Configuration {
    #[bits(8, default=0x0, access=RO)]
    rsvd: u8,
    #[bits(2, default=HumidityResolution::FourteenBit)]
    pub humidity_resolution: HumidityResolution,
    #[bits(1, default=TemperatureResolution::FourteenBit)]
    pub temperature_resolution: TemperatureResolution,
    #[bits(1, access=RO)]
    pub power_ok: bool,
    #[bits(1, default = AcquisitionMode::Both)]
    pub mode: AcquisitionMode,
    #[bits(1, default = false)]
    pub heater_enable: bool,
    #[bits(1, default=0, access=RO)]
    rsvd2: bool,
    #[bits(1, default = false)]
    pub reset: bool,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Acquisition mode for the HDC3022 sensor.
pub enum AcquisitionMode {
    #[default]
    /// Both temperature and humidity are acquired in sequence.
    Both = 0b0,
    /// Temperature and humidity are acquired separately.
    Separate = 0b1,
}

impl AcquisitionMode {
    pub(crate) const fn from_bits(bits: u8) -> Self {
        match bits {
            0b0 => AcquisitionMode::Both,
            0b1 => AcquisitionMode::Separate,
            _ => panic!("Invalid AcquisitionMode bits"),
        }
    }

    pub(crate) const fn into_bits(self) -> u8 {
        match self {
            AcquisitionMode::Both => 0b0,
            AcquisitionMode::Separate => 0b1,
        }
    }
}

impl Hdc3022Register for Configuration {
    const ADDRESS: u8 = 0x2;

    const REGISTER_LEN: usize = 2;

    fn read<T: I2c<SevenBitAddress>>(
        &mut self,
        hdc: &mut Hdc3022<T>,
    ) -> Result<(), Error<T::Error>> {
        let mut buffer = [0u8; Self::REGISTER_LEN];
        hdc.i2c
            .write_read(hdc.address, &[Self::ADDRESS], &mut buffer)?;
        *self = u16::from_be_bytes(buffer).into();
        Ok(())
    }

    fn write<T: I2c<SevenBitAddress>>(
        &mut self,
        hdc: &mut Hdc3022<T>,
    ) -> Result<(), Error<T::Error>> {
        let buffer = self.into_bits().to_be_bytes();
        hdc.i2c
            .write(hdc.address, &[Self::ADDRESS, buffer[0], buffer[1]])?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
/// Humidity measurement resolution for the HDC3022 sensor.
pub enum HumidityResolution {
    /// 8-bit resolution, with a conversion time of 2.5 milliseconds.
    EightBit = 0b10,
    /// 11-bit resolution, with a conversion time of 3.85 milliseconds.
    ElevenBit = 0b01,
    #[default]
    /// 14-bit resolution, with a conversion time of 6.5 milliseconds.
    FourteenBit = 0b00,
}

impl HumidityResolution {
    pub(crate) const fn from_bits(bits: u8) -> Self {
        match bits {
            0b10 => HumidityResolution::EightBit,
            0b01 => HumidityResolution::ElevenBit,
            0b00 => HumidityResolution::FourteenBit,
            _ => panic!("Invalid HumidityResolution bits"),
        }
    }

    pub(crate) const fn into_bits(self) -> u8 {
        match self {
            HumidityResolution::EightBit => 0b10,
            HumidityResolution::ElevenBit => 0b01,
            HumidityResolution::FourteenBit => 0b00,
        }
    }

    /// Returns the delay time in microseconds for the given humidity resolution.
    pub(crate) fn delay_time(self) -> u32 {
        match self {
            HumidityResolution::EightBit => 2500,
            HumidityResolution::ElevenBit => 3850,
            HumidityResolution::FourteenBit => 6500,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
/// Temperature measurement resolution for the HDC3022 sensor.
pub enum TemperatureResolution {
    /// 11-bit resolution, with a conversion time of 3.65 milliseconds.
    ElevenBit = 0b1,
    #[default]
    /// 14-bit resolution, with a conversion time of 6.35 milliseconds.
    FourteenBit = 0b0,
}

impl TemperatureResolution {
    pub(crate) const fn from_bits(bits: u8) -> Self {
        match bits {
            0b1 => TemperatureResolution::ElevenBit,
            0b0 => TemperatureResolution::FourteenBit,
            _ => panic!("Invalid TemperatureResolution bits"),
        }
    }

    pub(crate) const fn into_bits(self) -> u8 {
        match self {
            TemperatureResolution::ElevenBit => 0b1,
            TemperatureResolution::FourteenBit => 0b0,
        }
    }

    /// Returns the delay time in microseconds for the given temperature resolution.
    pub(crate) fn delay_time(self) -> u32 {
        match self {
            TemperatureResolution::ElevenBit => 3650,
            TemperatureResolution::FourteenBit => 6350,
        }
    }
}

#[derive(Debug, Default)]
pub struct SerialId(u64);

impl SerialId {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl Hdc3022Register for SerialId {
    const ADDRESS: u8 = 0xFB;
    const REGISTER_LEN: usize = 6;

    fn read<T: I2c<SevenBitAddress>>(
        &mut self,
        hdc: &mut Hdc3022<T>,
    ) -> Result<(), Error<T::Error>> {
        let mut buffer = [0u8; Self::REGISTER_LEN];
        hdc.i2c
            .write_read(hdc.address, &[Self::ADDRESS], &mut buffer)?;
        self.0 = (buffer[0] as u64) << 33
            | (buffer[1] as u64) << 25
            | (buffer[2] as u64) << 17
            | (buffer[3] as u64) << 9
            | (buffer[4] as u64) << 1
            | (buffer[5] as u64) >> 7;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct ManufacturerId(u16);

impl Hdc3022Register for ManufacturerId {
    const ADDRESS: u8 = 0xFE;
    const REGISTER_LEN: usize = 2;

    fn read<T: I2c<SevenBitAddress>>(
        &mut self,
        hdc: &mut Hdc3022<T>,
    ) -> Result<(), Error<T::Error>> {
        let mut buffer = [0u8; Self::REGISTER_LEN];
        hdc.i2c
            .write_read(hdc.address, &[Self::ADDRESS], &mut buffer)?;
        self.0 = u16::from_be_bytes(buffer);
        if self.0 != HDC3022_MANUFACTURER_ID {
            return Err(Error::InvalidId);
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct DeviceId(u16);

impl Hdc3022Register for DeviceId {
    const ADDRESS: u8 = 0xFF;
    const REGISTER_LEN: usize = 2;

    fn read<T: I2c<SevenBitAddress>>(
        &mut self,
        hdc: &mut Hdc3022<T>,
    ) -> Result<(), Error<T::Error>> {
        let mut buffer = [0u8; Self::REGISTER_LEN];
        hdc.i2c
            .write_read(hdc.address, &[Self::ADDRESS], &mut buffer)?;
        self.0 = u16::from_be_bytes(buffer);
        if self.0 != HDC3022_DEVICE_ID {
            return Err(Error::InvalidId);
        }
        Ok(())
    }
}
