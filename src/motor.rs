use crate::state::{Action, ActionRx, Event, EventTx};
use crate::utils;


use motor_controller::{DCMotor, MOTOR1, PCA9685};
use log::{debug, error, info};

use rppal::gpio::{Gpio, Trigger};
use rppal::i2c::I2c;
use tokio::sync::watch;
use tokio::task;

#[derive(Clone, Copy, Debug)]
enum DoorStatus {
    Open,
    Opening,
    Closed,
    Closing,
    Unknown,
}

pub fn create_motor_task(
    mut rx: ActionRx,
    mut event_tx: EventTx
) -> task::JoinHandle<()> {
    task::spawn_local(async move {
        let i2c = I2c::new().expect("Unable to open I2C bus.");
        let pca = PCA9685::new(i2c).unwrap();
        let mut motor = DCMotor::new(pca).unwrap();
        let gpios = Gpio::new().unwrap();

        // Run like this.
        motor.set_throttle(MOTOR1, Some(0.1));

        motor.set_throttle(MOTOR1, None);

        let (door_status_tx, mut door_status_rx) = watch::channel(DoorStatus::Unknown);

        let open_pin = gpios.get(22).unwrap().into_input_pulldown();
        let close_pin = gpios.get(23).unwrap().into_input_pulldown();
        
        let open_pin_task = utils::watch_pin(open_pin, Trigger::Both, rx.clone(), move |level| {

        });

        let close_pin_task = utils::watch_pin(close_pin, Trigger::Both, rx.clone(), move |level| {

        });

    })
}