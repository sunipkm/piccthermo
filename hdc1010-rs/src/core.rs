use core::time::Duration;

use embedded_hal::{
    delay::DelayNs,
    i2c::{I2c, SevenBitAddress},
};

use crate::{
    Error, Humidity, Temperature,
    address::SlaveAddress,
    register::{
        self, AcquisitionModeEnum, Configuration, DeviceId, Hdc1010Register, HumidityResolution,
        ManufacturerId, TemperatureResolution, Trigger,
    },
};

/// Represents the HDC1010 sensor.
pub struct Hdc1010<M> {
    pub(crate) address: u8,
    pub(crate) hres: HumidityResolution,
    pub(crate) tres: TemperatureResolution,
    pub(crate) trig: M,
}

#[derive(Debug, Default)]
/// Builder for a HDC1010 sensor.
pub struct Hdc1010Builder {
    pub(crate) address: SlaveAddress,
    pub(crate) hres: HumidityResolution,
    pub(crate) tres: TemperatureResolution,
}

/// Trait for acquisition modes of the HDC1010 sensor.
pub trait AcquisitionMode {
    /// The acquisition configuration value for the mode.
    const MODE: AcquisitionModeEnum;
}

/// Acquire humidity and temperature data in separate measurements.
pub struct Separate(Trigger);
impl AcquisitionMode for Separate {
    const MODE: AcquisitionModeEnum = AcquisitionModeEnum::Separate;
}

/// Acquire temperature and humidity data with a single command.
pub struct Both;
impl AcquisitionMode for Both {
    const MODE: AcquisitionModeEnum = AcquisitionModeEnum::Both;
}

impl Hdc1010Builder {
    /// Set the address of the HDC1010 sensor.
    pub fn with_address(mut self, address: SlaveAddress) -> Self {
        self.address = address;
        self
    }

    /// Set the humidity resolution for the HDC1010 sensor.
    pub fn with_humidity_resolution(mut self, resolution: HumidityResolution) -> Self {
        self.hres = resolution;
        self
    }

    /// Set the temperature resolution for the HDC1010 sensor.
    pub fn with_temperature_resolution(mut self, resolution: TemperatureResolution) -> Self {
        self.tres = resolution;
        self
    }
}

impl Hdc1010Builder {
    /// Build the HDC1010 sensor with the specified configuration.
    pub fn build_mode_both<T: I2c<SevenBitAddress>>(
        self,
        i2c: &mut T,
    ) -> Result<Hdc1010<Both>, Error<T::Error>> {
        let mut dev = Hdc1010 {
            address: self.address.into_bits(),
            hres: self.hres,
            tres: self.tres,
            trig: Both,
        };
        // Check if the device is present by reading its ID register
        let mut mfg = ManufacturerId::default();
        mfg.read(&mut dev, i2c)?;
        let mut dev_id = DeviceId::default();
        dev_id.read(&mut dev, i2c)?;
        let mut cfg = Configuration::default();
        cfg.read(&mut dev, i2c)?;
        cfg.set_mode(Both::MODE);
        cfg.set_humidity_resolution(self.hres);
        cfg.set_temperature_resolution(self.tres);
        cfg.write(&mut dev, i2c)?;
        Ok(dev)
    }
}

impl Hdc1010Builder {
    /// Build the HDC1010 sensor with the specified configuration.
    pub fn build_mode_separate<T: I2c<SevenBitAddress>>(
        self,
        i2c: &mut T,
    ) -> Result<Hdc1010<Separate>, Error<T::Error>> {
        let mut dev = Hdc1010 {
            address: self.address.into_bits(),
            hres: self.hres,
            tres: self.tres,
            trig: Separate(Trigger::Temperature),
        };
        // Check if the device is present by reading its ID register
        let mut mfg = ManufacturerId::default();
        mfg.read(&mut dev, i2c)?;
        let mut dev_id = DeviceId::default();
        dev_id.read(&mut dev, i2c)?;
        let mut cfg = Configuration::default();
        cfg.read(&mut dev, i2c)?;
        cfg.set_mode(Separate::MODE);
        cfg.set_humidity_resolution(self.hres);
        cfg.set_temperature_resolution(self.tres);
        cfg.write(&mut dev, i2c)?;
        Ok(dev)
    }
}

impl<U: AcquisitionMode> Hdc1010<U> {
    /// Get the current temperature and humidity resolutions.
    pub fn get_resolution(&mut self) -> (HumidityResolution, TemperatureResolution) {
        (self.hres, self.tres)
    }

    /// Get the address of the device.
    pub fn get_address(&self) -> u8 {
        self.address
    }

    /// Set the humidity and temperature resolutions.
    pub fn set_resolution<T: I2c<SevenBitAddress>>(
        &mut self,
        i2c: &mut T,
        humidity_resolution: HumidityResolution,
        temperature_resolution: TemperatureResolution,
    ) -> Result<(), Error<T::Error>> {
        let mut conf = Configuration::default();
        conf.set_humidity_resolution(humidity_resolution);
        conf.set_temperature_resolution(temperature_resolution);
        conf.write(self, i2c)?;
        conf.read(self, i2c)?;
        self.hres = conf.humidity_resolution();
        self.tres = conf.temperature_resolution();
        Ok(())
    }

    /// Set the heater state of the HDC1010 sensor.
    pub fn set_heater<T: I2c<SevenBitAddress>>(
        &mut self,
        i2c: &mut T,
        enable: bool,
    ) -> Result<(), Error<T::Error>> {
        let mut conf = Configuration::default();
        conf.read(self, i2c)?;
        conf.set_heater_enable(enable);
        conf.write(self, i2c)?;
        Ok(())
    }

    /// Get the heater state of the HDC1010 sensor.
    pub fn get_heater<T: I2c<SevenBitAddress>>(
        &mut self,
        i2c: &mut T,
    ) -> Result<bool, Error<T::Error>> {
        let mut conf = Configuration::default();
        conf.read(self, i2c)?;
        Ok(conf.heater_enable())
    }

    /// Get the power status of the HDC1010 sensor.
    pub fn get_power_status<T: I2c<SevenBitAddress>>(
        &mut self,
        i2c: &mut T,
    ) -> Result<bool, Error<T::Error>> {
        let mut conf = Configuration::default();
        conf.read(self, i2c)?;
        Ok(conf.power_ok())
    }

    /// Get the serial number of the HDC1010 sensor.
    pub fn get_serial<T: I2c<SevenBitAddress>>(
        &mut self,
        i2c: &mut T,
    ) -> Result<u64, Error<T::Error>> {
        let mut serial = register::SerialId::default();
        serial.read(self, i2c)?;
        Ok(serial.value())
    }

    /// Perform a soft reset of the HDC1010 sensor.
    pub fn reset<T: I2c<SevenBitAddress>, D: DelayNs>(
        &mut self,
        i2c: &mut T,
        delay: &mut D,
    ) -> Result<(), Error<T::Error>> {
        let mut conf = Configuration::default();
        conf.set_reset(true);
        conf.write(self, i2c)?;
        for _ in 0..10 {
            delay.delay_ms(500);
            conf.read(self, i2c)?;
            if !conf.reset() {
                break;
            }
        }
        if conf.reset() {
            return Err(Error::Timeout);
        }
        // Reconfigure the device with the current settings
        conf.set_reset(false);
        conf.set_mode(U::MODE);
        conf.set_humidity_resolution(self.hres);
        conf.set_temperature_resolution(self.tres);
        conf.write(self, i2c)?;
        conf.read(self, i2c)?;
        self.hres = conf.humidity_resolution();
        self.tres = conf.temperature_resolution();
        Ok(())
    }

    /// Get the builder for the HDC1010 sensor.
    /// This allows you to change the acquisition mode.
    pub fn to_builder(self) -> Hdc1010Builder {
        Hdc1010Builder {
            address: SlaveAddress::from_bits(self.address),
            hres: self.hres,
            tres: self.tres,
        }
    }
}

impl Hdc1010<Both> {
    /// Trigger a measurement of temperature, humidity, or both.
    ///
    /// # Parameters:
    /// - `kind`: An optional [`Trigger`] enum that specifies whether to measure temperature, humidity, or both.
    ///   Note: If the acquisition mode is not set to [`AcquisitionMode::Both`] while trigger is [`Trigger::Both`], an error is returned.
    ///
    /// # Returns:
    /// - [`Duration`]: The duration to wait for the measurement to complete.
    pub fn trigger<T: I2c<SevenBitAddress>>(
        &mut self,
        i2c: &mut T,
    ) -> Result<Duration, Error<T::Error>> {
        let delay = self.hres.delay_time() + self.tres.delay_time();
        Temperature::default().write(self, i2c)?;
        Ok(Duration::from_micros(delay as _))
    }

    /// Read the current temperature value.
    pub fn read_temperature_humidity<T: I2c<SevenBitAddress>>(
        &mut self,
        i2c: &mut T,
    ) -> Result<(Temperature, Humidity), Error<T::Error>> {
        let mut buf = [0u8; 4];
        i2c.read(self.address, &mut buf)?;
        let temp = Temperature {
            value: u16::from_be_bytes([buf[0], buf[1]]),
        };
        let hum = Humidity {
            value: u16::from_be_bytes([buf[2], buf[3]]),
        };
        Ok((temp, hum))
    }
}

impl Hdc1010<Separate> {
    /// Trigger a measurement of temperature, humidity, or both.
    ///
    /// # Parameters:
    /// - `kind`: An optional [`Trigger`] enum that specifies whether to measure temperature, humidity, or both.
    ///   Note: If the acquisition mode is not set to [`AcquisitionMode::Separate`] while trigger is [`Trigger::Both`], an error is returned.
    ///
    /// # Returns:
    /// - [`Duration`]: The duration to wait for the measurement to complete.
    pub fn trigger<T: I2c<SevenBitAddress>>(
        &mut self,
        i2c: &mut T,
        kind: Trigger,
    ) -> Result<Duration, Error<T::Error>> {
        let delay = match kind {
            Trigger::Temperature => {
                Temperature::default().write(self, i2c)?;
                self.tres.delay_time()
            }
            Trigger::Humidity => {
                Humidity::default().write(self, i2c)?;
                self.hres.delay_time()
            }
        };
        self.trig.0 = kind;
        Ok(Duration::from_micros(delay as _))
    }

    /// Read the current temperature value.
    pub fn read_temperature<T: I2c<SevenBitAddress>>(
        &mut self,
        i2c: &mut T,
    ) -> Result<Temperature, Error<T::Error>> {
        if self.trig.0 != Trigger::Temperature {
            return Err(Error::InvalidOperation);
        }
        let mut v = Temperature::default();
        v.read(self, i2c)?;
        Ok(v)
    }

    /// Read the current humidity value.
    pub fn read_humidity<T: I2c<SevenBitAddress>>(
        &mut self,
        i2c: &mut T,
    ) -> Result<Humidity, Error<T::Error>> {
        if self.trig.0 != Trigger::Humidity {
            return Err(Error::InvalidOperation);
        }
        let mut v = Humidity::default();
        v.read(self, i2c)?;
        Ok(v)
    }
}
