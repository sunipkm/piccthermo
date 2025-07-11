use bitfield_struct::bitfield;

#[bitfield(u8)]
/// Represents the slave address for the HDC3022 sensor.
/// The address is 7 bits long, with the least significant bit (LSB) used for read/write operations.
/// The default address is 0x44, which is the standard I2C address for the HDC3022.
/// The address can be configured by setting the `a0` and `a1` bits.
pub struct SlaveAddress {
    #[bits(1, default = false)]
    pub a0: bool,
    #[bits(1, default = false)]
    pub a1: bool,
    #[bits(6, default = 0x44)]
    reserved: u8,
}
