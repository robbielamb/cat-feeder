use crate::state::{Action, ActionRx, Event, EventTx};

use ads1015_adc::*;
use log::{debug, error, info, warn};

use rppal::i2c::I2c;
use tokio::task;

pub fn distance_task(mut rx: ActionRx, _event_tx: EventTx) -> task::JoinHandle<()> {
    task::spawn_local( async move {
        let i2c = I2c::new().expect("Unable to open I2C bus.");
        let mut adc = ADS1015::new(i2c).unwrap();

        loop {
            match rx.recv().await {
                None|Some(Action::Shutdown) => {
                    debug!("Shut down distance task");
                    break },
                Some(Action::Startup) => debug!("Task in startup mode"),
                _ => {
                    if let Ok(value) = adc.read(Pin::P0)  {
                        info!("Value is: {}", value)
                    }
                }
            }
        }
    })
}