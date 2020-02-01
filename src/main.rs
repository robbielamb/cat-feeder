// Standard imports
#![recursion_limit = "256"]
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Server};

use futures::join;

use futures::{
    future::FutureExt, // for `.fuse()`
    select,
};

use log::{debug, error, info};

use rppal::gpio::{Gpio, Level, Trigger};

/* use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::prelude::*; */
use tokio::runtime::Runtime;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{mpsc, watch, Mutex};
use tokio::task;
use tokio::time::delay_for;

// Local Code
mod assets;

mod config;

mod camera;
use camera::create_picture_task;

mod distance;
use distance::create_distance_task;

mod motor;

mod result;
use result::Result;

mod rfid_reader;

mod state;
use state::{reducer_task, ActionRx, ActionTx, Event, EventRx, EventTx, State};

mod utils;

mod http;
use crate::http::service;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let config = config::read_config();

    let addr: SocketAddr = config.listen_port.parse()?;

    //let addr: SocketAddr = "0.0.0.0:1337".parse()?;

    let mut rt = Runtime::new()?;
    let gpios = Gpio::new()?;

    let local = task::LocalSet::new();

    let (tx, rx): (EventTx, EventRx) = mpsc::unbounded_channel::<Event>();

    let (action_tx, mut action_rx): (ActionTx, ActionRx) = watch::channel(state::Action::Startup);

    let state = Arc::new(tokio::sync::Mutex::new(State::new()));

    let _ = local.block_on(&mut rt, async move {
        let reducer_task = reducer_task(Arc::clone(&state), rx, action_tx);

        let picture_task = create_picture_task(action_rx.clone(), tx.clone());

        let looping_task = looping_state(tx.clone(), action_rx.clone(), Arc::clone(&state));

        let rfid_reader_task = rfid_reader::rfid_reader(tx.clone(), action_rx.clone());

        let distance_task = create_distance_task(action_rx.clone(), config.distance, tx.clone());

        let motor_task = motor::create_motor_task(action_rx.clone(), tx.clone());

        let button = gpios.get(20).unwrap().into_input_pulldown();
        let button_tx = tx.clone();
        let button_listener =
            utils::watch_pin(button, Trigger::Both, action_rx.clone(), move |i| match i {
                Level::High => {
                    info!("Caught a high edge here");
                    if let Err(err) = button_tx.send(Event::IncClick) {
                        error!("Error sending click: {}", err)
                    }
                }
                Level::Low => {
                    info!("Caught a low edge here");
                    ()
                }
            });

        let service_tx = tx.clone();
        let clone_state = Arc::clone(&state);
        let make_service = make_service_fn(move |_| {
            let clone_state = Arc::clone(&clone_state);
            let service_tx = service_tx.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |request: Request<Body>| {
                    let state = Arc::clone(&clone_state);
                    let tx = service_tx.clone();

                    service::http_response(request, state, tx)
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_service);
        let server = server.with_graceful_shutdown(async move {
            debug!("In the quitting service");
            loop {
                if let Some(state::Action::Shutdown) = action_rx.recv().await {
                    debug!("HTTP Recieved quit event");
                    break;
                }
            }
            debug!("Quitting HTTP");
            ()
        });

        //let quit_listener_tx = tx.clone();
        let quit_listener = task::spawn(async move {
            debug!("Installing signal handler");
            // An infinite stream of hangup signals.
            let mut stream = signal(SignalKind::interrupt()).unwrap();
            stream.recv().await;
            debug!("got signal HUP. Asking tasks to shut down");
            if let Err(_err) = tx.send(Event::Shutdown) {
                error!("Error broadcasting shutdown");
            }
            debug!("Quitting quit listener");
            ()
        });

        info!("Starting Services");

        let _ret = tokio::join!(
            button_listener,
            distance_task,
            looping_task,
            motor_task,
            picture_task,
            quit_listener,
            reducer_task,
            rfid_reader_task,
            server,
        );
    });

    Ok(())
}

/// A simple task that increments a counter ever 5 seconds
fn looping_state(
    tx: EventTx,
    mut stop_rx: watch::Receiver<state::Action>,
    state: Arc<Mutex<State>>,
) -> task::JoinHandle<Result<()>> {
    task::spawn(async move {
        loop {
            {
                if let Err(_err) = tx.send(Event::IncLoop) {
                    error!("Error sending message");
                }
                let state = state.lock().await;
                info!("In the spawned loop {} times", state.loop_count);
            }
            // Either wait for 5 seconds or wait for the quit message
            select! {
                _ = Box::pin(delay_for(Duration::from_secs(5)).fuse()) => (),
                recv = stop_rx.recv().fuse() => if let Some(state::Action::Shutdown) = recv {
                    debug!("Shutting down looper");
                    break }
            }
        }
        Ok(())
    })
}
