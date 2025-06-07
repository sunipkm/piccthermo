use core::time::Duration;

use embedded_hal::{
    delay::DelayNs,
    i2c::{I2c, SevenBitAddress},
};

use crate::{
    AcquisitionMode, Error, Humidity, Temperature,
    address::SlaveAddress,
    register::{
        self, Configuration, DeviceId, Hdc1010Register, HumidityResolution, ManufacturerId,
        TemperatureResolution, Trigger,
    },
};

/// Represents the HDC1010 sensor.
pub struct Hdc1010<'a, T> {
    pub(crate) i2c: &'a mut T,
    pub(crate) address: u8,
    pub(crate) mode: AcquisitionMode,
    pub(crate) hres: HumidityResolution,
    pub(crate) tres: TemperatureResolution,
}

#[derive(Debug, Default)]
/// Builder for a HDC1010 sensor.
pub struct Hdc1010Builder {
    pub(crate) address: SlaveAddress,
    pub(crate) mode: AcquisitionMode,
    pub(crate) hres: HumidityResolution,
    pub(crate) tres: TemperatureResolution,
}

impl Hdc1010Builder {
    /// Set the address of the HDC1010 sensor.
    pub fn with_address(mut self, address: SlaveAddress) -> Self {
        self.address = address;
        self
    }

    /// Set the acquisition mode for the HDC1010 sensor.
    pub fn with_mode(mut self, mode: AcquisitionMode) -> Self {
        self.mode = mode;
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

    /// Build the HDC1010 sensor with the specified configuration.
    pub fn build<T: I2c<SevenBitAddress>>(
        self,
        i2c: &mut T,
    ) -> Result<Hdc1010<'_, T>, Error<T::Error>> {
        let mut dev = Hdc1010 {
            i2c,
            address: self.address.into_bits(),
            mode: self.mode,
            hres: self.hres,
            tres: self.tres,
        };
        // Check if the device is present by reading its ID register
        let mut mfg = ManufacturerId::default();
        mfg.read(&mut dev)?;
        let mut dev_id = DeviceId::default();
        dev_id.read(&mut dev)?;
        let mut cfg = Configuration::default();
        cfg.read(&mut dev)?;
        cfg.set_mode(self.mode);
        cfg.set_humidity_resolution(self.hres);
        cfg.set_temperature_resolution(self.tres);
        cfg.write(&mut dev)?;
        Ok(dev)
    }
}

impl<T: I2c<SevenBitAddress>> Hdc1010<'_, T> {
    /// Get the current temperature and humidity resolutions.
    pub fn get_resolution(&mut self) -> (HumidityResolution, TemperatureResolution) {
        (self.hres, self.tres)
    }

    /// Set the humidity and temperature resolutions.
    pub fn set_resolution(
        &mut self,
        humidity_resolution: HumidityResolution,
        temperature_resolution: TemperatureResolution,
    ) -> Result<(), Error<T::Error>> {
        let mut conf = Configuration::default();
        conf.set_humidity_resolution(humidity_resolution);
        conf.set_temperature_resolution(temperature_resolution);
        conf.write(self)?;
        conf.read(self)?;
        self.hres = conf.humidity_resolution();
        self.tres = conf.temperature_resolution();
        Ok(())
    }

    /// Get the current acquisition mode of the HDC1010 sensor.
    pub fn get_mode(&mut self) -> AcquisitionMode {
        self.mode
    }

    /// Set the acquisition mode of the HDC1010 sensor.
    pub fn set_mode(&mut self, mode: AcquisitionMode) -> Result<(), Error<T::Error>> {
        let mut conf = Configuration::default();
        conf.read(self)?;
        conf.set_mode(mode);
        conf.write(self)?;
        conf.read(self)?;
        self.mode = mode;
        Ok(())
    }

    /// Set the heater state of the HDC1010 sensor.
    pub fn set_heater(&mut self, enable: bool) -> Result<(), Error<T::Error>> {
        let mut conf = Configuration::default();
        conf.read(self)?;
        conf.set_heater_enable(enable);
        conf.write(self)?;
        Ok(())
    }

    /// Get the power status of the HDC1010 sensor.
    pub fn get_power_status(&mut self) -> Result<bool, Error<T::Error>> {
        let mut conf = Configuration::default();
        conf.read(self)?;
        Ok(conf.power_ok())
    }

    /// Get the serial number of the HDC1010 sensor.
    pub fn get_serial(&mut self) -> Result<u64, Error<T::Error>> {
        let mut serial = register::SerialId::default();
        serial.read(self)?;
        Ok(serial.value())
    }

    /// Perform a soft reset of the HDC1010 sensor.
    pub fn reset<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), Error<T::Error>> {
        let mut conf = Configuration::default();
        conf.set_reset(true);
        conf.write(self)?;
        for _ in 0..10 {
            delay.delay_ms(500);
            conf.read(self)?;
            if !conf.reset() {
                break;
            }
        }
        if conf.reset() {
            return Err(Error::Timeout);
        }
        // Reconfigure the device with the current settings
        conf.set_reset(false);
        conf.set_mode(self.mode);
        conf.set_humidity_resolution(self.hres);
        conf.set_temperature_resolution(self.tres);
        conf.write(self)?;
        conf.read(self)?;
        self.mode = conf.mode();
        self.hres = conf.humidity_resolution();
        self.tres = conf.temperature_resolution();
        Ok(())
    }

    /// Trigger a measurement of temperature, humidity, or both.
    ///
    /// # Parameters:
    /// - `kind`: An optional [`Trigger`] enum that specifies whether to measure temperature, humidity, or both.
    ///   Note: If the acquisition mode is not set to [`AcquisitionMode::Both`] while trigger is [`Trigger::Both`], an error is returned.
    ///
    /// # Returns:
    /// - [`Duration`]: The duration to wait for the measurement to complete.
    pub fn trigger(&mut self, kind: Trigger) -> Result<Duration, Error<T::Error>> {
        let mut delay = 0;

        match kind {
            Trigger::Both => {
                if self.mode != AcquisitionMode::Both {
                    return Err(Error::InvalidOperation);
                }
                delay += self.hres.delay_time() + self.tres.delay_time();
                Temperature::default().read(self)?
            }
            Trigger::Temperature => {
                delay += self.tres.delay_time();
                if self.mode == AcquisitionMode::Both {
                    delay += self.hres.delay_time();
                }
                Temperature::default().read(self)?
            }
            Trigger::Humidity => {
                if self.mode == AcquisitionMode::Both {
                    delay += self.tres.delay_time();
                    Temperature::default().read(self)? // Trigger using temperature in case acquisition mode is Both
                } else {
                    Humidity::default().read(self)?; // Trigger using humidity if acquisition mode is Separate
                }
                delay += self.hres.delay_time();
            }
        }

        Ok(Duration::from_micros(delay as _))
    }

    /// Read the current humidity value.
    pub fn read_humidity(&mut self) -> Result<Humidity, Error<T::Error>> {
        let mut humidity = Humidity::default();
        humidity.read(self)?;
        Ok(humidity)
    }

    /// Read the current temperature value.
    pub fn read_temperature(&mut self) -> Result<Temperature, Error<T::Error>> {
        let mut temperature = Temperature::default();
        temperature.read(self)?;
        Ok(temperature)
    }
}
