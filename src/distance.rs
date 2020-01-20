use crate::config::{Config, Distance};
use crate::state::{Action, ActionRx, Event, EventTx};
use crate::utils;

use ads1015_adc::*;
use futures::{
    future::FutureExt, // for `.fuse()`
    select,
};
use log::{debug, error, info};

use rppal::gpio::{Gpio, Trigger};
use rppal::i2c::I2c;
use tokio::sync::watch;
use tokio::task;
use tokio::time::delay_for;

use std::time::Duration;

#[derive(Clone, Copy, Debug)]
enum Conversion {
    Ready,
    NotReady,
}

pub fn distance_task(
    mut rx: ActionRx,
    distance_config: Distance,
    _event_tx: EventTx,
) -> task::JoinHandle<()> {
    task::spawn_local(async move {
        let i2c = I2c::new().expect("Unable to open I2C bus.");
        let mut adc = ADS1015::new(i2c).unwrap();
        let gpios = Gpio::new().unwrap();

        adc.gain = Gain::Gain2;
        //adc.data_rate = SampleRate::Rate920;

        adc.set_alert_status().unwrap();

        let (send_conversion_ready, mut recieve_conversion_ready) =
            watch::channel(Conversion::NotReady);

        let pin = gpios
            .get(distance_config.alert_pin)
            .unwrap()
            .into_input_pulldown();

        let pin_watcher = utils::watch_pin(pin, Trigger::RisingEdge, rx.clone(), move |x| {
            info!("Pin triggered: {:?}", x);
            if let Err(x) = send_conversion_ready.broadcast(Conversion::Ready) {
                error!("Error broadcasting pin conversion ready: {:?}", x);
            }
        });

        let mut delay = delay_for(Duration::from_millis(distance_config.interval)).fuse();

        //pin_mut!(pin_watcher);
        let mut pin_watcher = pin_watcher.fuse();
        loop {
            select! {
                foo = pin_watcher => {},
                conversion_event = recieve_conversion_ready.recv().fuse() => {
                    let value = adc.read_conversion().unwrap();
                    debug!("Distance Value {}", value);

                    delay = delay_for(Duration::from_millis(distance_config.interval)).fuse();
                    // Do something with the value now
                }
                _ = delay => {
                    adc.request_read(Pin::P0).unwrap();
                }
                action = rx.recv().fuse() => {
                    match action {
                       Some(Action::Shutdown) => {
                            debug!("Shut down distance task");
                            break;
                        }
                        Some(Action::Startup) => debug!("Task in startup mode"),
                        _ => {
                         // Nothing
                        }
                    }
                }
            }
        }
    })
}
