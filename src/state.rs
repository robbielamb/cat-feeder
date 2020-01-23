use std::sync::Arc;

use log::{debug, error};

use tokio::sync::{mpsc, watch, Mutex};
use tokio::task;


/// Shorthand for the transmit half of the event message channel.
pub type EventTx = mpsc::UnboundedSender<Event>;

/// Shorthand for the receive half of the event message channel.
pub type EventRx = mpsc::UnboundedReceiver<Event>;

/// Shorthand for the send half of the broadcast channel.
pub type ActionTx = watch::Sender<Action>;

/// Shorthand for the recieve half of the broadcast channel.
pub type ActionRx = watch::Receiver<Action>;

/// The state of the application
pub struct State {
    pub click_count: u32,
    pub distance: u16,
    in_threshold: bool,
    last_tag_read: Option<u32>,
    pub loop_count: u32,
    pub has_camera: bool,
    taking_picture: bool,
    pub pictures: Vec<Vec<u8>>,
}

impl State {
    pub fn new() -> Self {
        State {
            click_count: 0,
            distance: 0,
            in_threshold: false,
            last_tag_read: None,
            loop_count: 0,
            has_camera: false,
            taking_picture: false,
            pictures: vec![],
        }
    }

    pub fn last_tag_read(&self) -> Option<u32> {
        self.last_tag_read
    }
}

/// Events that happen from the outside world. These are items that would
/// cause the state to be updated.
pub enum Event {
    /// Increment the Click Accumliator
    IncClick,
    /// Increment the Loop Accumliator
    IncLoop,
    /// Last Tag to be read
    ReadTag(u32),
    /// Register a Camera or Not
    HasCamera(bool),
    /// Add an image to the list of imasges
    AddImage(Vec<u8>),
    /// External request to take an image with the camera
    TakeImageRequest,
    /// Request an image be deleted from the image list
    DeleteImage(usize),
    /// Endering the configured distance threshold
    EnterDistanceThreshold(u16),
    /// Notification of the distance
    Distance(u16),
    /// Exiting the configured distance threshold
    ExitDistanceThreshold(u16),
    /// Event requesting everything shut down
    Shutdown,
}

/// Actions are a response to the state being updated after an event.
/// They tell other parts of the application to update based on a new state.
/// Right now this is to take a picture or shutdown. In the future this can also
/// ask lights to blink or a motor to turn.
#[derive(Clone, Copy)]
pub enum Action {
    /// Default action when app is starting up
    Startup,
    /// Action to captue an image with the camera
    TakePicture,
    /// Action to shut down all tasks
    Shutdown,
}

// Consumes the event along with a state.
// Updates the state object and sends out actions to take.
async fn reducer(event: Event, state: &Mutex<State>, action_tx: &ActionTx) {
    match event {
        Event::IncLoop => {
            state.lock().await.loop_count += 1;
        }
        Event::IncClick => {
            state.lock().await.click_count += 1;
        }
        Event::ReadTag(tag) => state.lock().await.last_tag_read = Some(tag),
        Event::HasCamera(camera) => {
            state.lock().await.has_camera = camera;
        }
        Event::TakeImageRequest => {
            let mut state = state.lock().await;
            if state.has_camera && !state.taking_picture {
                state.taking_picture = true;
                if let Err(_err) = action_tx.broadcast(Action::TakePicture) {
                    error!("Error sending take picture");
                }
            } else {
                debug!("Image Taking request with no camera");
            }
        }
        Event::AddImage(image) => {
            debug!("Saving image to memory");
            let mut state = state.lock().await;
            let picture_count = state.pictures.len();
            if picture_count >= 80 {
                let _ = state.pictures.remove(0);
            }

            state.pictures.push(image);
            state.taking_picture = false;
        }
        Event::DeleteImage(id) => {
            let mut state = state.lock().await;
            if state.pictures.len() > (id) {
                let _ = state.pictures.remove(id);
            }
        }
        Event::EnterDistanceThreshold(distance) => {
            let mut state = state.lock().await;
            state.distance = distance;
            state.in_threshold = true;
            if state.has_camera && !state.taking_picture {
                state.taking_picture = true;
                if let Err(_err) = action_tx.broadcast(Action::TakePicture) {
                    error!("Error sending take picture");
                }
            }
        }
        Event::Distance(distance) => {
            let mut state = state.lock().await;
            state.distance = distance;
        }
        Event::ExitDistanceThreshold(distance) => {
            let mut state = state.lock().await;
            state.distance = distance;
            state.in_threshold = false;
            if state.has_camera && !state.taking_picture {
                state.taking_picture = true;
                if let Err(_err) = action_tx.broadcast(Action::TakePicture) {
                    error!("Error sending take picture");
                }
            }
        }
        Event::Shutdown => {
            if let Err(_err) = action_tx.broadcast(Action::Shutdown) {
                error!("Error shutting down");
            }
        }
    };
}

///
pub fn reducer_task(
    state_handle: Arc<Mutex<State>>,
    mut rx: EventRx,
    mut action_tx: ActionTx,   
) -> task::JoinHandle<()> {
    task::spawn(async move {
        // rx.recv() returns None when all TXs are shutdown
        while let Some(event) = rx.recv().await {
            reducer(event, &state_handle, &action_tx).await
        }
        debug!("All Recievers dropped");
        // This will stall until all RX side have been shutdown
        action_tx.closed().await;
        debug!("All recivers dropped. Quitting now");
    })
}
