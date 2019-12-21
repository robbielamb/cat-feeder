// Standard imports
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

// https://github.com/djc/askama
use askama::Template;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};

use futures_util::future::join3;
use log::{debug, error, info, trace, warn};
use rascam;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::{mpsc, Mutex};
use tokio::task;
use tokio::time::delay_for;
use tokio::time::delay_for;
use url::form_urlencoded;

// Local Code
mod assets;
use assets::Image;
mod http_utils;
mod result;
//use result::error::Error;
use result::Result;

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
    click_count: &'a u32,
    loop_count: &'a u32,
}

struct Shared {
    click_count: u32,
    loop_count: u32,
    camera_enabled: bool,
}

impl Shared {
    fn new() -> Self {
        Shared {
            click_count: 0,
            loop_count: 0,
            camera_enabled: false,
        }
    }
}

enum Event {
    IncClick,
    IncLoop,
}

/// Shorthand for the transmit half of the message channel.
type Tx = mpsc::UnboundedSender<Event>;

/// Shorthand for the receive half of the message channel.
type Rx = mpsc::UnboundedReceiver<Event>;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    info!("Logging");
    warn!("Warning");
    let addr = "0.0.0.0:1337".parse()?;

    let (tx, mut rx): (Tx, Rx) = mpsc::unbounded_channel::<Event>();

    let tx: Arc<Mutex<Tx>> = Arc::new(Mutex::new(tx));

    // For the most basic of state, we just share a counter, that increments
    // with each request, and we send its value back in the response.
    let state = Arc::new(Mutex::new(Shared::new()));

    let reducer_state = Arc::clone(&state);
    let reducer_task = task::spawn(async move {
        while let Some(event) = rx.recv().await {
            //let state = state.lock().await;
            reducer(event, &reducer_state).await
        }
    });

    let service_tx = Arc::clone(&tx);
    let clone_state = Arc::clone(&state);
    let make_service = make_service_fn(move |_| {
        let clone_state = Arc::clone(&clone_state);
        let service_tx = Arc::clone(&service_tx);
        async move {
            Ok::<_, hyper::Error>(service_fn(move |request: Request<Body>| {
                //let count = counter.fetch_add(1, Ordering::AcqRel);

                let state = Arc::clone(&clone_state);
                let tx = Arc::clone(&service_tx);
                test_response(request, state, tx)
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_service);

    let my_task_tx = Arc::clone(&tx);
    let my_task = task::spawn(async move {
        //let state = Arc::clone(&state);
        loop {
            {
                if let Err(_err) = my_task_tx.lock().await.send(Event::IncLoop) {
                    error!("Error sending message");
                }
                let state = state.lock().await;
                //state.loop_count += 1;
                info!("In the spawned loop {} times", state.loop_count);
            }
            delay_for(Duration::from_secs(5)).await;
        }
    });

    info!("Starting Server");

    let _ret = join3(reducer_task, my_task, server).await;

    Ok(())
}

async fn test_response(
    req: Request<Body>,
    state: Arc<Mutex<Shared>>,
    tx: Arc<Mutex<Tx>>,
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

            if let Err(err) = tx.lock().await.send(Event::IncClick) {
                error!("Error sending click event: {}", err)
            }
            http_utils::redirect_to("/".to_string())
        }
        (&Method::GET, &["favicon.ico"]) => http_utils::get_png("cat-icon_64.png"),
        (&Method::GET, &["hello", x]) => {
            let state = state.lock().await;
            let hello = HelloTemplate {
                name: x,
                click_count: &state.click_count,
                loop_count: &state.loop_count,
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

async fn reducer(event: Event, state: &Mutex<Shared>) {
    match event {
        Event::IncLoop => {
            state.lock().await.loop_count += 1;
        }
        Event::IncClick => {
            state.lock().await.click_count += 1;
        }
    };
}

async fn take_picture(info: &rascam::CameraInfo) -> Result<()> {
    let mut camera = rascam::SimpleCamera::new(info.clone())?;
    camera.activate()?;

    delay_for(Duration::from_millis(2000)).await;

    let picture = camera.take_one_async().await?;

    Ok()
}
