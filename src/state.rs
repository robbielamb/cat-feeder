use std::sync::Arc;

use futures::{
    future::FutureExt, // for `.fuse()`
    select,
};
use log::debug;

use tokio::sync::{mpsc, watch, Mutex};
use tokio::task;

/// Shorthand for the transmit half of the message channel.
pub type Tx = mpsc::UnboundedSender<Event>;

/// Shorthand for the receive half of the message channel.
pub type Rx = mpsc::UnboundedReceiver<Event>;

pub struct Shared {
    pub click_count: u32,
    last_tag_read: Option<u32>,
    pub loop_count: u32,
    pub has_camera: bool,
    pub pictures: Vec<Vec<u8>>,
}

impl Shared {
    pub fn new() -> Self {
        Shared {
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

async fn reducer(event: Event, state: &Mutex<Shared>) {
    match event {
        Event::IncLoop => {
            state.lock().await.loop_count += 1;
        }
        Event::IncClick => {
            state.lock().await.click_count += 1;
        }
        Event::ReadTag(tag) => {
            state.lock().await.last_tag_read = Some(tag)
        }
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
    state_handle: Arc<Mutex<Shared>>,
    mut rx: Rx,
    mut stop_rx: watch::Receiver<RunState>,
) -> task::JoinHandle<()> {
    task::spawn(async move {
        /*  while let Some(event) = rx.recv().await {
            reducer(event, &state_handle).await
        } */

        loop {
            select! {
                event = rx.recv().fuse() => {
                    if let Some(event) = event {
                        reducer(event, &state_handle).await
                    }
                }
                event = stop_rx.recv().fuse() => if let Some(RunState::Shutdown) = event {
                    debug!("Ending reducer task");
                    break
                }
            }
        }
    })
}

#[derive(Clone, Copy)]
pub enum RunState {
    Run,
    Shutdown,
}
