use crate::pca9685_pwc;
use crate::pca9685_pwc::PCA9685;
use crate::Error;

use log::trace;

use std::error;
use std::f32;
use std::f32::consts::PI;

//const STEPPER1_CHANNELS: [u8; 4] = [10, 9, 11, 12];
const STEPPER1_CHANNELS: [u8; 4] = [9, 11, 10, 12];

//const _STEPPER2_CHANNELS: [u8; 4] = [4, 3, 5, 6];
const _STEPPER2_CHANNELS: [u8; 4] = [3, 5, 4, 6];

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Style {
    Single,
    Double,
    Interleave,
    Microstep,
}

type Result<T> = std::result::Result<T, Error>;

pub struct StepperMotor {
    pub pca: PCA9685,
    current_micro_step: u16,
    micro_steps: u16,
    curve: Vec<u16>,
}

impl StepperMotor {
    pub fn new(pca: PCA9685, micro_steps: u16) -> Result<StepperMotor> {
        let total_steps: usize = (micro_steps + 1) as usize;
        let mut curve: Vec<u16> = Vec::with_capacity(total_steps);
        for i in 0..total_steps {
            let step = (0xffff as f32 * (PI / (2.0 * micro_steps as f32) * (i as f32)).sin())
                .round() as u16;
            curve.push(step);
        }

        trace!("Computed Curves: {:?}", curve);

        let motor = StepperMotor {
            pca,
            current_micro_step: 0,
            micro_steps,
            curve,
        };

        motor.update_coils(false)?;

        Ok(motor)
    }

    pub fn release(&self) -> Result<()> {
        for i in 0..4 {
            self.pca.set_duty_cycle(STEPPER1_CHANNELS[i], 0xffff)?;
        }
        Ok(())
    }

    fn update_coils(&self, microstepping: bool) -> Result<()> {
        let duty_cycles = &mut [0; 4];
        let trailing_coil = ((self.current_micro_step / self.micro_steps) % 4) as usize;
        let leading_coil = ((trailing_coil + 1) % 4) as usize;
        let microstep = (self.current_micro_step % self.micro_steps) as usize;
        duty_cycles[leading_coil] = self.curve[microstep];
        duty_cycles[trailing_coil] = self.curve[(self.micro_steps as usize - microstep)];

        if !microstepping
            && (duty_cycles[leading_coil] == duty_cycles[trailing_coil]
                && duty_cycles[leading_coil] > 0)
        {
            duty_cycles[leading_coil] = 0xffff;
            duty_cycles[trailing_coil] = 0xffff;
        };

        trace!(
            "Duty Cycle: {},{},{},{}",
            duty_cycles[0],
            duty_cycles[1],
            duty_cycles[2],
            duty_cycles[3]
        );

        // Engage Coils!
        for i in 0..4 {
            self.pca
                .set_duty_cycle(STEPPER1_CHANNELS[i], duty_cycles[i])?;
        }

        Ok(())
    }

    pub fn onestep(&mut self, direction: Direction, style: Style) -> Result<u16> {
        let mut step_size = 0;

        if style == Style::Microstep {
            step_size = 1;
        } else {
            let half_step = self.micro_steps / 2;
            let full_step = self.micro_steps;

            let additional_microsteps = self.current_micro_step % half_step;
            if additional_microsteps != 0 {
                if direction == Direction::Forward {
                    self.current_micro_step += half_step + additional_microsteps;
                } else {
                    self.current_micro_step -= additional_microsteps
                }
                step_size = 0;
            } else if style == Style::Interleave {
                step_size = half_step;
            };

            let current_interleave = self.current_micro_step / half_step;
            if (style == Style::Single && current_interleave % 2 == 1)
                || (style == Style::Double && current_interleave % 2 == 0)
            {
                step_size = half_step;
            } else if style == Style::Single || style == Style::Double {
                step_size = full_step;
            };
        };

        if direction == Direction::Forward {
            self.current_micro_step += step_size;
        } else {
            self.current_micro_step -= step_size;
        };

        self.update_coils(style == Style::Microstep)?;

        Ok(self.current_micro_step)
    }
}
