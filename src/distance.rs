use crate::config::Distance;
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
use tokio::time::{delay_for, Duration};

#[derive(Clone, Copy, Debug)]
enum Conversion {
    Ready,
    NotReady,
}

pub fn create_distance_task(
    mut rx: ActionRx,
    distance_config: Distance,
    mut event_tx: EventTx,
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

        let mut in_threshold: bool = false;
        let mut pin_watcher = pin_watcher.fuse();
        loop {
            select! {
                // Just to run the pin watcher. It will quit on it's own.
                _ = pin_watcher => {},
                // Request when the conversion pin triggers
                _conversion_event = recieve_conversion_ready.recv().fuse() => {
                    let value = adc.read_conversion().unwrap();
                    evaluate_value(value, &distance_config, &mut event_tx, &mut in_threshold);
                    // Reset the delay for when to trigger the pin again
                    delay = delay_for(Duration::from_millis(distance_config.interval)).fuse();
                }
                // Request the pin be read async.
                _ = delay => {
                    adc.request_read(Pin::P0).unwrap();
                }
                action = rx.recv().fuse() => {
                    match action {
                       Some(Action::Shutdown) => {
                            debug!("Shut down distance task");
                            break;
                        }
                        Some(Action::Startup) => debug!("Distance Task in startup mode"),
                        _ => {
                         // Nothing
                        }
                    }
                }
            }
        }
    })
}

// Decide what to do with the value read from the ADC given the config. Possibly send
// and event.
fn evaluate_value(
    value: u16,
    distance_config: &Distance,
    event_tx: &mut EventTx,
    in_threshold: &mut bool,
) {
    info!("Distance value: {}", value);
    if value >= distance_config.enter_threshold && *in_threshold == false {
        *in_threshold = true;
        if let Err(err) = event_tx.send(Event::EnterDistanceThreshold(value)) {
            error!("Error sending event: {}", err);
        }
    } else if value < distance_config.exit_threshold && *in_threshold == true {
        *in_threshold = false;
        if let Err(err) = event_tx.send(Event::ExitDistanceThreshold(value)) {
            error!("Error sending event: {}", err);
        }
    } else {
        if let Err(err) = event_tx.send(Event::Distance(value)) {
            error!("Error sending event: {}", err);
        }
    }
}
