#[derive(Debug)]
/// Represents errors that can occur while interacting with the HDC1010 sensor.
pub enum Error<E> {
    /// An error occurred while communicating with the I2C bus.
    I2c(E),
    /// An error occurred while reading or writing to the sensor.
    InvalidAddress,
    /// An error occurred due to an invalid ID.
    InvalidId,
    /// Attempted to write to a register that is not writable.
    ReadOnly,
    /// An error occurred due to an invalid operation.
    Timeout,
}

impl<E> From<E> for Error<E> {
    fn from(e: E) -> Self {
        Error::I2c(e)
    }
}
