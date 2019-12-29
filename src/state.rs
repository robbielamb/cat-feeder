use std::sync::Arc;

use futures::{
    future::FutureExt, // for `.fuse()`
    select,
};
use log::debug;

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

pub struct State {
    pub click_count: u32,
    last_tag_read: Option<u32>,
    pub loop_count: u32,
    pub has_camera: bool,
    pub pictures: Vec<Vec<u8>>,
}

impl State {
    pub fn new() -> Self {
        State {
            click_count: 0,
            last_tag_read: None,
            loop_count: 0,
            has_camera: false,
            pictures: vec![],
        }
    }

    pub fn last_tag_read(&self) -> Option<u32> {
        self.last_tag_read
    }
}

pub enum Event {
    IncClick,
    IncLoop,
    ReadTag(u32),
    HasCamera(bool),
    AddImage(Vec<u8>),
}

async fn reducer(event: Event, state: &Mutex<State>) {
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
        Event::AddImage(image) => {
            debug!("Saving image to memory");
            state.lock().await.pictures.push(image);
        }
    };
}

pub fn reducer_task(
    state_handle: Arc<Mutex<State>>,
    mut rx: EventRx,
    mut stop_rx: watch::Receiver<Action>,
) -> task::JoinHandle<()> {
    task::spawn(async move {
        loop {
            select! {
                event = rx.recv().fuse() => {
                    if let Some(event) = event {
                        reducer(event, &state_handle).await
                    }
                }
                event = stop_rx.recv().fuse() => if let Some(Action::Shutdown) = event {
                    debug!("Ending reducer task");
                    break
                }
            }
        }
    })
}

#[derive(Clone, Copy)]
pub enum Action {
    Run,
    TakePicture,
    Shutdown,
}
