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
use camera::{picture_task, Picture, Rx as PictRx, Tx as PictTx};

mod state;
use state::{reducer_task, Event, Rx, Shared, Tx};

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

    let mut rt = Runtime::new()?;

    let addr = "0.0.0.0:1337".parse()?;

    let local = task::LocalSet::new();
    //let pictures_tx = Arc::clone(&tx);
    let _unknown_tasks = local.block_on(&mut rt, async move {
        let (tx, rx): (Tx, Rx) = mpsc::unbounded_channel::<Event>();

        let (pict_tx, pict_rx): (PictTx, PictRx) = mpsc::unbounded_channel::<Picture>();

        let (mut stop_tx, mut stop_rx) = watch::channel(state::RunState::Run);

        let picture_task = picture_task(pict_rx, tx.clone());

        let state = Arc::new(Mutex::new(Shared::new()));

        let reducer_state = Arc::clone(&state);
        let reducer_task = reducer_task(reducer_state, rx, stop_rx.clone());

        let looping_task = looping_state(tx.clone(), stop_rx.clone(), Arc::clone(&state));

        let rfid_reader_task = rfid_reader::rfid_reader(tx.clone());

        //let tx: Arc<Mutex<Tx>> = Arc::new(Mutex::new(tx));

        let service_tx = tx.clone();
        let clone_state = Arc::clone(&state);
        let make_service = make_service_fn(move |_| {
            let clone_state = Arc::clone(&clone_state);
            let service_tx = service_tx.clone();
            let pict_tx = pict_tx.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |request: Request<Body>| {
                    let state = Arc::clone(&clone_state);
                    let tx = service_tx.clone();

                    let pict_tx = pict_tx.clone();
                    test_response(request, state, tx, pict_tx)
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_service);
        //let mut http_serv_stop = stop_rx.clone();
        let server = server.with_graceful_shutdown(async move {
            debug!("In the quitting service");
            while let Some(state::RunState::Run) = stop_rx.recv().await {}
            ()
        });

      

        let quit_listener = task::spawn_local(async move {
            debug!("Installing signal handler");
            // An infinite stream of hangup signals.
            let mut stream = signal(SignalKind::interrupt()).unwrap();
            stream.recv().await;
            debug!("got signal HUP. Asking tasks to shut down");
            if let Err(_err) = stop_tx.broadcast(state::RunState::Shutdown) {
                error!("Error broadcasting shutdown");
            }
            stop_tx.closed().await;
            debug!("Quitting quit listener");
            ()
        });

        info!("Starting Services");

        let _ret = join!(
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

fn looping_state(
    tx: Tx,
    mut stop_rx: watch::Receiver<state::RunState>,
    state: Arc<Mutex<Shared>>,
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
                recv = stop_rx.recv().fuse() => if let Some(state::RunState::Shutdown) = recv {
                    debug!("Shutting down looper");
                    break }
            }
            //delay_for(Duration::from_secs(5)).await;
        }
    })
}

async fn test_response(
    req: Request<Body>,
    state: Arc<Mutex<Shared>>,
    tx: Tx,
    pict_tx: PictTx,
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
                    if let Err(err) = pict_tx.send(Picture::Take) {
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
