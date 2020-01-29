use log::trace;
use rppal::i2c;

use std::error;
use std::f32;
use std::fmt;
use std::thread;
use std::time::Duration;

// The board can be stacked an configured for addresses 0x60-0x80
const PCA9685_DEFAULT_ADDRESS: u16 = 0x60;

/// Address for communicating to several PCA9685 devices at the same time
const _PCA9685_ALL_ADDRESS: u16 = 0x70;

const MODE1_REG: u8 = 0x00;
const _MODE2_REG: u8 = 0x01;

const PRESCALE_REG: u8 = 0xFE;

/// Errors when accessing the PCA9685
#[derive(Debug)]
pub enum Error {
    /// Errors that occur when accessing the I2C peripheral.    
    I2cError(i2c::Error),
    // Error when setting a frequency out of bounds.
    FrequencyError,
    // Error when using a channel out of bounds
    ChannelError(u8),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::FrequencyError => write!(f, "Frequency must be 3 or greater."),
            Error::ChannelError(channel) => {
                write!(f, "Channel must be between 0 and 15. {} was used.", channel)
            }
            Error::I2cError(ref error) => error.fmt(f),
        }
    }
}

impl error::Error for Error {}

impl From<i2c::Error> for Error {
    fn from(err: i2c::Error) -> Error {
        Error::I2cError(err)
    }
}

type Result<T> = std::result::Result<T, Error>;

pub struct PCA9685 {
    i2c: i2c::I2c,
    reference_clock_speed: u32,
}

impl PCA9685 {
    /// Construct a new PCA9685 on the given i2c bus.
    pub fn new(i2c: i2c::I2c) -> Result<PCA9685> {
        let mut obj = PCA9685 {
            i2c,
            reference_clock_speed: 25_000_000,
        };

        obj.i2c.set_slave_address(PCA9685_DEFAULT_ADDRESS)?;

        obj.reset()?;

        Ok(obj)
    }

    /// Reset the chip.
    /// I'm not sure this is a good way to do this
    pub fn reset(&self) -> Result<()> {
        self.set_mode1(0x00)?;

        Ok(())
    }

    /// The overall PWM frequency in Hertz.
    pub fn frequency(&self) -> Result<u32> {
        Ok((self.reference_clock_speed / 4096) / self.get_prescale()? as u32)
    }

    /// Set the overall PMW frequency in Hertz.
    /// Reasonable numbers are from 24-1526
    pub fn set_frequency(&self, frequency: u32) -> Result<()> {
        let prescale: u8 =
            (self.reference_clock_speed as f32 / (4096.0 * frequency as f32)).round() as u8;
        if prescale < 3 {
            return Err(Error::FrequencyError);
        }
        let old_mode = self.get_mode1()?;
        // Writes to the prescale register are blocked when not sleeping
        let sleep_mode = (old_mode & 0x7F) | 0x10; // Mode 1 sleep
        self.set_mode1(sleep_mode)?;
        self.set_prescale(prescale)?;
        self.set_mode1(old_mode)?; // Restore the original mode, without sleep
        thread::sleep(Duration::from_millis(5)); // Wake back up!
                                                 // Should Enabling autoincement happen somewhere else?
        self.set_mode1(old_mode | 0xa1)?; // Enable auto increment

        Ok(())
    }

    pub fn get_prescale(&self) -> Result<u8> {
        Ok(self.i2c.smbus_read_byte(PRESCALE_REG)?)
    }

    fn set_prescale(&self, byte: u8) -> Result<()> {
        self.i2c.smbus_write_byte(PRESCALE_REG, byte)?;
        Ok(())
    }

    /// Get the value of the mode1 register
    pub fn get_mode1(&self) -> Result<u8> {
        Ok(self.i2c.smbus_read_byte(MODE1_REG)?)
    }

    /// Set the value of the mode1 register
    fn set_mode1(&self, byte: u8) -> Result<()> {
        self.i2c.smbus_write_byte(MODE1_REG, byte)?;
        Ok(())
    }

    /// Autoincrement through the registers as they are being written
    pub fn enable_autoincrement(&self) -> Result<()> {
        let old_mode = self.get_mode1()?;
        self.set_mode1(old_mode | 0xa1)?;
        Ok(())
    }

    pub fn get_pwm(&self, chan: u8) -> Result<(u16, u16)> {
        channel_in_range(chan)?;
        let reg = 0x06 + 4 * chan;
        let write_buffer: &[u8] = &[reg];
        let read_buffer: &mut [u8; 4] = &mut [0; 4];

        self.i2c.write_read(write_buffer, read_buffer)?;
        // Flip the bits for the 12 bit numbers
        let on = (read_buffer[1] as u16) << 8 | read_buffer[0] as u16;
        let off = (read_buffer[3] as u16) << 8 | read_buffer[2] as u16;

        Ok((on, off))
    }

    pub fn set_pwm(&self, chan: u8, on: u16, off: u16) -> Result<()> {
        channel_in_range(chan)?;
        let reg = 0x06 + 4 * chan;

        let buf: &mut [u8; 4] = &mut [0; 4];
        // The low byte comes first, so repack this into our byte array
        buf[0] = (on & 0xFF) as u8;
        buf[1] = ((on & 0xFF00) >> 8) as u8;
        buf[2] = (off & 0xFF) as u8;
        buf[3] = ((off & 0xFF00) >> 8) as u8;

        trace!(
            "{} on: {:#x},{:#x} off: {:#x},{:#x}",
            reg,
            buf[0],
            buf[1],
            buf[2],
            buf[3]
        );

        //self.i2c.smbus_block_write(reg, buf)?;
        self.i2c.block_write(reg, buf)?;

        Ok(())
    }

    /// Magic adafruit stuff? I guess this is for driving the motor
    pub fn get_duty_cycle(&self, chan: u8) -> Result<u16> {
        let (on, off) = self.get_pwm(chan)?;

        return if on == 0x1000 {
            Ok(0xffff)
        } else {
            Ok(off << 4)
        };
    }

    /// Magic adafruit stuff? I guess this is for driving the motor
    pub fn set_duty_cycle(&self, chan: u8, value: u16) -> Result<()> {
        if value == 0xffff {
            self.set_pwm(chan, 0x1000, 0)?
        } else {
            let value = (value + 1) >> 4;
            self.set_pwm(chan, 0, value)?
        }
        Ok(())
    }
}

/// check to see if the channel is valid
/// Valid channels are 0-15
fn channel_in_range(chan: u8) -> Result<()> {
    if chan > 15 {
        Err(Error::ChannelError(chan))
    } else {
        Ok(())
    }
}
