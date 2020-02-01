use crate::pca9685_pwc;
use crate::pca9685_pwc::PCA9685;
use crate::Error;

type Result<T> = std::result::Result<T, Error>;

type MotorAddrs = (u8, u8);

pub const MOTOR1: MotorAddrs = (9, 10); // disable 8?
pub const MOTOR2: MotorAddrs = (11, 12); // disable 13?
pub const MOTOR3: MotorAddrs = (3, 4); // disable 2?
pub const MOTOR4: MotorAddrs = (5, 6); // disable 7?

pub struct DCMotor {
    pub pca: PCA9685,
}

impl DCMotor {
    /// Create a new DC Motor.
    pub fn new(pca: PCA9685) -> Result<DCMotor> {
        Ok(DCMotor { pca })
    }

    pub fn set_throttle(&self, motor: MotorAddrs, throttle: Option<f32>) -> Result<()> {
        let cycles = match throttle {
            None => (0, 0),
            Some(value) if value == 0.0 => (0xFFFF, 0xFFFF),
            Some(value) => {
                let duty_cycle: u16 = (value.abs() * (0xFFFF as f32)) as u16;

                if value < 0.0 {
                    (0, duty_cycle)
                } else {
                    (duty_cycle, 0)
                }
            }
        };

        self.set_cycles(motor, cycles)?;
        Ok(())
    }

    fn set_cycles(&self, motor: MotorAddrs, cycles: (u16, u16)) -> Result<()> {
        self.pca.set_duty_cycle(motor.0, cycles.0)?;
        self.pca.set_duty_cycle(motor.0, cycles.0)?;
        Ok(())
    }
}
