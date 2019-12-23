// Standard imports
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

// https://github.com/djc/askama
use askama::Template;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};

use futures::join;
use futures_util::future::join3;
use log::{debug, error, info, trace, warn};
use rascam;

use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::prelude::*;
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, Mutex};
use tokio::task;
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
    picture_count: &'a usize,
}

struct Shared {
    click_count: u32,
    loop_count: u32,
    has_camera: bool,
    pictures: Vec<Vec<u8>>,
}

impl Shared {
    /* fn new() -> Self {
        Self::new_with_camera(None)
    } */

    fn new() -> Self {
        Shared {
            click_count: 0,
            loop_count: 0,
            has_camera: false,
            pictures: vec![],
        }
    }
}

enum Event {
    IncClick,
    IncLoop,
    HasCamera(bool),
    AddImage(Vec<u8>),
}

/// Shorthand for the transmit half of the message channel.
type Tx = mpsc::UnboundedSender<Event>;

/// Shorthand for the receive half of the message channel.
type Rx = mpsc::UnboundedReceiver<Event>;

enum Picture {
    Take,
}

type PictTx = mpsc::UnboundedSender<Picture>;
type PictRx = mpsc::UnboundedReceiver<Picture>;

//#[tokio::main]
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    info!("Logging");
    warn!("Warning");
    let mut rt = Runtime::new()?;
    let addr = "0.0.0.0:1337".parse()?;

    let (tx, mut rx): (Tx, Rx) = mpsc::unbounded_channel::<Event>();

    let tx: Arc<Mutex<Tx>> = Arc::new(Mutex::new(tx));

    let (pict_tx, mut pict_rx): (PictTx, PictRx) = mpsc::unbounded_channel::<Picture>();

    let pict_tx: Arc<Mutex<PictTx>> = Arc::new(Mutex::new(pict_tx));

    let local = task::LocalSet::new();
    let pictures_tx = Arc::clone(&tx);
    let _picture_task = local.block_on(&mut rt, async move {
        let picture_task = task::spawn_local(async move {
            debug!("Starting picture task");
            let mut camera;
            let camera_info = match rascam::info() {
                Ok(info) => {
                    if info.cameras.len() < 1 {
                        warn!("No cameras found on device");
                        None
                    } else {
                        if let Err(err) = pictures_tx.lock().await.send(Event::HasCamera(true)) {
                            error!("Error sending click event: {}", err)
                        }
                        debug!("We have a camera");
                        camera = rascam::SimpleCamera::new(info.cameras[0].clone()).unwrap();
                        camera.activate().unwrap();
                        std::thread::sleep(Duration::from_millis(2000));
                        Some(camera)
                    }
                }
                Err(err) => {
                    error!("Error opening camera: {}", err);
                    None
                }
            };
            if let Some(mut camera) = camera_info {
                while let Some(_) = pict_rx.recv().await {
                    debug!("Request for a picture");
                    let picture = camera.take_one();
                    match picture {
                        Ok(pict) => {
                            if let Err(err) = pictures_tx.lock().await.send(Event::AddImage(pict)) {
                                error!("Error saving picture: {}", err)
                            }
                        }
                        Err(err) => error!("Error taking picture: {}", err),
                    }
                }
            }
            debug!("Ending picture task");
        });
       
    //});

    ///////////////////////////////////////////////////////////////

    //rt.block_on(async {


        // Should capture this error...
        /*      let camera_info = match rascam::info() {
            Ok(info) => {
                if info.cameras.len() < 1 {
                    warn!("No cameras found on device");
                    None
                } else {
                    Some(info.cameras[0])
                }
            },
            Err(err) => {error!("Error opening camera: {}", err); None},
        }; */
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
        let pict_tx = Arc::clone(&pict_tx);
        let clone_state = Arc::clone(&state);
        let make_service = make_service_fn(move |_| {
            let clone_state = Arc::clone(&clone_state);
            let service_tx = Arc::clone(&service_tx);
            let pict_tx = Arc::clone(&pict_tx);
            async move {
                Ok::<_, hyper::Error>(service_fn(move |request: Request<Body>| {
                    //let count = counter.fetch_add(1, Ordering::AcqRel);

                    let state = Arc::clone(&clone_state);
                    let tx = Arc::clone(&service_tx);
                    let pict_tx = Arc::clone(&pict_tx);
                    test_response(request, state, tx, pict_tx)
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

        let _ret = join!(reducer_task, my_task, server, picture_task);

       
    });

    Ok(())
}

async fn test_response(
    req: Request<Body>,
    state: Arc<Mutex<Shared>>,
    tx: Arc<Mutex<Tx>>,
    pict_tx: Arc<Mutex<PictTx>>,
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
        (&Method::POST, &["take_picture"]) => {
            let has_camera = { state.lock().await.has_camera };
            match has_camera {
                false => http_utils::not_found(), // Should have a nice message about taking a picture or not
                true => {
                    // Send message to take picture
                    if let Err(err) = pict_tx.lock().await.send(Picture::Take) {
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
        (&Method::GET, &["picture", numb]) => {
            match numb.parse::<usize>() {
                Ok(i) => {
                    let picts = &state.lock().await.pictures;
                    if picts.len() > 0 && i < picts.len()  {
                        let some_pict = &picts[i];
                        http_utils::get_camera_image(some_pict.to_vec())                       
                    } else {
                        http_utils::not_found() 
                    }
                }
                Err(_e) => http_utils::not_found()
            }
         /*    let picts = &state.lock().await.pictures;
            if picts.len() < 1 {
                http_utils::not_found()
            } else {
                let some_pict = &picts[picts.len() - 1];
                http_utils::get_camera_image(some_pict.to_vec())
            } */
        }
        (&Method::GET, &["hello", x]) => {
            let state = state.lock().await;
            let hello = HelloTemplate {
                name: x,
                click_count: &state.click_count,
                loop_count: &state.loop_count,
                picture_count: &state.pictures.len(),
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

/* fn take_picture() -> Result<Vec<u8>> {
    let mut camera = rascam::SimpleCamera::new(info.clone())?;
    camera.activate()?;

    std::thread::sleep(Duration::from_millis(2000));

    debug!("Taking picture");
    let picture = camera.take_one()?;

    Ok(picture)
} */
