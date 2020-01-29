use rppal::i2c::I2c;

use pca9685_pwc::PCA9685;
use pca9685_pwc::{Direction, StepperMotor, Style};

use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    let i2c = I2c::new().expect("Unable to open I2C bus.");
    let pca = PCA9685::new(i2c)?;
    let mode = pca.get_mode1()?;
    println!("Mode at the beginning {:#b}", mode);
    pca.set_frequency(1600)?;
    let mut motor = StepperMotor::new(pca, 16)?;

    let freq = motor.pca.frequency()?;
    println!("Frequency is {}", freq);
    let prescale = motor.pca.get_prescale()?;
    println!("Prescale is {}", prescale);

    for _ in 0..1300 {
        let steps = motor.onestep(Direction::Forward, Style::Single)?;
        println!("microsteps {}", steps);
        sleep(Duration::from_millis(5));
    }

    let mode = motor.pca.get_mode1()?;
    println!("Mode at the end {:#b}", mode);

    println!("Releasing the motor");
    motor.release()?;
    //motor.pca.reset()?;

    Ok(())
}
