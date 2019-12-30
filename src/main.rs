// Standard imports
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

// https://github.com/djc/askama
use askama::Template;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};

use futures::join;

use futures::{
    future::FutureExt, // for `.fuse()`
    select,
};

use log::{debug, error, info};

use rppal::gpio::{Gpio, InputPin, Level, Trigger};

/* use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::prelude::*; */
use tokio::runtime::Runtime;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{mpsc, watch, Mutex};
use tokio::task;
use tokio::time::delay_for;
use url::form_urlencoded;

// Local Code
mod assets;
//use assets::Image;
mod http_utils;
mod result;
use result::Result;

mod rfid_reader;

mod camera;
use camera::picture_task;

mod distance;
use distance::distance_task;

mod state;
use state::{reducer_task, ActionRx, ActionTx, Event, EventRx, EventTx, State};

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
    click_count: &'a u32,
    loop_count: &'a u32,
    picture_count: &'a usize,
    last_tag: u32,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let addr = "0.0.0.0:1337".parse()?;

    let mut rt = Runtime::new()?;
    let gpios = Gpio::new()?;

    let local = task::LocalSet::new();

    let (tx, rx): (EventTx, EventRx) = mpsc::unbounded_channel::<Event>();

    let (action_tx, mut action_rx): (ActionTx, ActionRx) = watch::channel(state::Action::Startup);

    let state = Arc::new(Mutex::new(State::new()));

    let _ = local.block_on(&mut rt, async move {
        let reducer_task = reducer_task(Arc::clone(&state), rx, action_tx);

        let picture_task = picture_task(action_rx.clone(), tx.clone());

        let looping_task = looping_state(tx.clone(), action_rx.clone(), Arc::clone(&state));

        let rfid_reader_task = rfid_reader::rfid_reader(tx.clone(), action_rx.clone());

        let distance_task = distance_task(action_rx.clone(), tx.clone());

        let button = gpios.get(20).unwrap().into_input_pulldown();
        let button_tx = tx.clone();
        let button_listener = watch_pin(button, action_rx.clone(), move |i| match i {
            Level::High => {
                info!("Caught a highm edge here");
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

                    test_response(request, state, tx)
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

        let _ret = join!(
            button_listener,
            distance_task,
            looping_task,
            picture_task,
            quit_listener,
            reducer_task,
            rfid_reader_task,
            server,
        );
    });

    Ok(())
}

// A simple task that increments a counter ever 5 seconds
fn looping_state(
    tx: EventTx,
    mut stop_rx: watch::Receiver<state::Action>,
    state: Arc<Mutex<State>>,
) -> task::JoinHandle<()> {
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
    })
}

// The webserver task
async fn test_response(
    req: Request<Body>,
    state: Arc<Mutex<State>>,
    tx: EventTx,
) -> Result<Response<Body>> {
    debug!("Pre Parse Path {:?}", req.uri().path());

    let path = req
        .uri()
        .path()
        .split("/")
        .filter(|x| x != &"")
        .collect::<Vec<&str>>();

    debug!("Requested path is: {:?}", path);

    //let baz = (req.method(), &path[..]);
    let headers = req.headers();
    debug!("Headers are {:?}", headers);

    match (req.method(), &path[..]) {
        (&Method::GET, &[]) => {
            let state = state.lock().await;
            let hello = HelloTemplate {
                name: "hey there",
                click_count: &state.click_count,
                loop_count: &state.loop_count,
                picture_count: &state.pictures.len(),
                last_tag: state.last_tag_read().unwrap_or(0),
            };
            let template = hello.render()?;
            http_utils::render_template(template)
        }
        (&Method::POST, &["increase_click"]) => {
            //let body = req.body();
            //let whole_body = hyper::body::aggregate(req).await?;
            let b = hyper::body::to_bytes(req).await?;
            debug!("The body is {:?}", b);
            // Parse the request body. form_urlencoded::parse
            // always succeeds, but in general parsing may
            // fail (for example, an invalid post of json), so
            // returning early with BadRequest may be
            // necessary.
            //
            // Warning: this is a simplified use case. In
            // principle names can appear multiple times in a
            // form, and the values should be rolled up into a
            // HashMap<String, Vec<String>>. However in this
            // example the simpler approach is sufficient.
            let params = form_urlencoded::parse(b.as_ref())
                .into_owned()
                .collect::<HashMap<String, String>>();

            debug!("Hashed Params are {:?}", params);

            if let Err(err) = tx.send(Event::IncClick) {
                error!("Error sending click event: {}", err)
            }
            http_utils::redirect_to("/".to_string())
        }
        (&Method::GET, &["favicon.ico"]) => http_utils::get_png("cat-icon_64.png"),
        (&Method::POST, &["take_picture"]) => {
            let has_camera = { state.lock().await.has_camera };
            match has_camera {
                false => http_utils::not_found(), // Should have a nice message about taking a picture or not
                true => {
                    // Send message to take picture
                    if let Err(err) = tx.send(Event::TakeImageRequest) {
                        error!("Error taking picture: {}", err);
                    };
                    http_utils::redirect_to("/latest_picture".to_string())
                }
            }
        }
        (&Method::GET, &["latest_picture"]) => {
            let picts = &state.lock().await.pictures;
            if picts.len() < 1 {
                http_utils::not_found()
            } else {
                let some_pict = &picts[picts.len() - 1];
                http_utils::get_camera_image(some_pict.to_vec())
            }
        }
        (&Method::GET, &["picture", numb]) => match numb.parse::<usize>() {
            Ok(i) => {
                let picts = &state.lock().await.pictures;
                if picts.len() > 0 && i < picts.len() {
                    let some_pict = &picts[i];
                    http_utils::get_camera_image(some_pict.to_vec())
                } else {
                    http_utils::not_found()
                }
            }
            Err(_e) => http_utils::not_found(),
        },
        (&Method::GET, &["hello", x]) => {
            let state = state.lock().await;
            let hello = HelloTemplate {
                name: x,
                click_count: &state.click_count,
                loop_count: &state.loop_count,
                picture_count: &state.pictures.len(),
                last_tag: state.last_tag_read().unwrap_or(0),
            };
            let template = hello.render()?;
            http_utils::render_template(template)
        }

        _ => {
            debug!("Not Found");
            http_utils::not_found()
        }
    }
}

// First stab at watching GPIO Events
// The async starts a new thread, so not ideal, but this does seem to work.
pub fn watch_pin<C>(mut pin: InputPin, mut action_rx: ActionRx, response: C) -> task::JoinHandle<()>
where
    C: FnMut(Level) + Send + 'static,
{
    task::spawn(async move {
        let _ = pin.set_async_interrupt(Trigger::Both, response);

        loop {
            if let Some(state::Action::Shutdown) = action_rx.recv().await {
                debug!("Shutting down Pin Task");
                break;
            }
        }
    })
}
