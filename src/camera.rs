use crate::state;

use std::time::Duration;

use log::{debug, error, warn};
//use rascam;
use tokio::sync::mpsc;
use tokio::task;
use tokio::time::delay_for;

pub enum Picture {
    Take,
}

pub type Tx = mpsc::UnboundedSender<Picture>;
pub type Rx = mpsc::UnboundedReceiver<Picture>;

pub fn picture_task(mut rx: Rx, state_tx: state::EventTx) -> task::JoinHandle<()> {
    task::spawn_local(async move {
        debug!("Starting picture task");
        let mut camera;
        let camera_info = match rascam::info() {
            Ok(info) => {
                if info.cameras.len() < 1 {
                    warn!("No cameras found on device");
                    None
                } else {
                    if let Err(err) = state_tx.send(state::Event::HasCamera(true)) {
                        error!("Error sending click event: {}", err)
                    }
                    debug!("We have a camera");
                    camera = rascam::SimpleCamera::new(info.cameras[0].clone()).unwrap();
                    camera.activate().unwrap();
                    delay_for(Duration::from_millis(2000)).await;
                    Some(camera)
                }
            }
            Err(err) => {
                error!("Error opening camera: {}", err);
                None
            }
        };
        if let Some(mut camera) = camera_info {
            while let Some(_) = rx.recv().await {
                debug!("Request for a picture");
                let picture = camera.take_one_async().await;
                match picture {
                    Ok(pict) => {
                        if let Err(err) = state_tx.send(state::Event::AddImage(pict)) {
                            error!("Error saving picture: {}", err)
                        }
                    }
                    Err(err) => error!("Error taking picture: {}", err),
                }
            }
        }
        debug!("Ending picture task");
    })
}
