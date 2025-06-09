// #![no_std]
#![deny(missing_docs)]
//!# HDC1010 - Driver for the Texas Instruments HDC1010 Humidity and Temperature Sensor
//! This crate provides a driver for the HDC1010 sensor, allowing you to read humidity and temperature data.
//! It supports various configurations such as acquisition mode and resolution settings.
mod address;
mod core;
mod error;
mod register;

pub use address::SlaveAddress;
pub use core::{Hdc1010, Hdc1010Builder};
pub use error::Error;
pub use register::{
    AcquisitionMode, Humidity, HumidityResolution, Temperature, TemperatureResolution, Trigger,
};
