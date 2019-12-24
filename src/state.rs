use std::sync::Arc;

use log::debug;

use tokio::sync::{mpsc, Mutex};
use tokio::task;

/// Shorthand for the transmit half of the message channel.
pub type Tx = mpsc::UnboundedSender<Event>;

/// Shorthand for the receive half of the message channel.
pub type Rx = mpsc::UnboundedReceiver<Event>;

pub struct Shared {
    pub click_count: u32,
    pub loop_count: u32,
    pub has_camera: bool,
    pub pictures: Vec<Vec<u8>>,
}

impl Shared {
    pub fn new() -> Self {
        Shared {
            click_count: 0,
            loop_count: 0,
            has_camera: false,
            pictures: vec![],
        }
    }
}

pub enum Event {
    IncClick,
    IncLoop,
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
        Event::HasCamera(camera) => {
            state.lock().await.has_camera = camera;
        }
        Event::AddImage(image) => {
            debug!("Saving image to memory");
            //let image = Box::new(image);
            //image.to
            state.lock().await.pictures.push(image);
        }
    };
}

pub fn reducer_task(state_handle: Arc<Mutex<Shared>>, mut rx: Rx) -> task::JoinHandle<()> {
    task::spawn(async move {
        while let Some(event) = rx.recv().await {
            reducer(event, &state_handle).await
        }
    })
}
