use log::debug;
use rppal::gpio::{InputPin, Level, Trigger};
use tokio::task;

use crate::state::{Action, ActionRx};

/// An adaptor for Watching the given GPIO pin and calling the response when it's triggered.
///
/// Takes ownership of the pin.
/// set_async_interrupt starts a new thread, so not ideal, but this does seem to workfor my use case
///
pub fn watch_pin<C>(
    mut pin: InputPin,
    trigger: Trigger,
    mut action_rx: ActionRx,
    response: C,
) -> task::JoinHandle<()>
where
    C: FnMut(Level) + Send + 'static,
{
    task::spawn(async move {
        let _ = pin.set_async_interrupt(trigger, response);

        loop {
            if let Some(Action::Shutdown) = action_rx.recv().await {
                debug!("Shutting down Pin Task");
                break;
            }
        }
    })
}
