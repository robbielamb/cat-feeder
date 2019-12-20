/// https://github.com/djc/askama
/// and hyper.rs
use askama::Template;

//#[macro_use] extern crate log;
use log::{debug, error, info, trace, warn};

use tokio::task;
//use tokio::time;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::{mpsc, Mutex};
use tokio::time::delay_for;

use futures_util::future::join3;

use std::time::Duration;

use std::sync::Arc;

//se hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};

#[derive(Debug)]
enum Error {
    BoringError,
    HyperError(hyper::error::Error),
    HttpError(http::Error),
    TemplateError(askama::Error),
}

impl std::error::Error for Error {}

impl std::convert::From<hyper::error::Error> for Error {
    fn from(err: hyper::error::Error) -> Self {
        Error::HyperError(err)
    }
}

impl std::convert::From<http::Error> for Error {
    fn from(err: http::Error) -> Self {
        Error::HttpError(err)
    }
}

impl std::convert::From<askama::Error> for Error {
    fn from(err: askama::Error) -> Self {
        Error::TemplateError(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BoringError => write!(f, "A boring Error"),
            Error::HyperError(err) => err.fmt(f),
            Error::HttpError(err) => err.fmt(f),
            Error::TemplateError(err) => err.fmt(f),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;


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
}

impl Shared {
    fn new() -> Self {
        Shared {
            click_count: 0,
            loop_count: 0,
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
    let addr = "0.0.0.0:1337".parse().unwrap();

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
    let state = state.lock().await;

    debug!("Pre Parse Path {:?}", req.uri().path());
    let path = req
        .uri()
        .path()
        .split("/")
        .filter(|x| x != &"")
        .collect::<Vec<&str>>();

    debug!("Requested path is: {:?}", path);

    let baz = (req.method(), &path[..]);

    match baz {
        (&Method::GET, &[]) => {
            if let Err(_err) = tx.lock().await.send(Event::IncClick) {
                error!("Error sending message");
            }
            let headers = req.headers();
            debug!("Headers are {:?}", headers);
            let hello = HelloTemplate {
                name: "hey there",
                click_count: &state.click_count,
                loop_count: &state.loop_count,
            };
            let template = hello.render().unwrap();
            Ok(respond_with_html().body(Body::from(template))?)
        }
        (&Method::GET, &["favicon.ico"]) => {
            if let Ok(mut file) = File::open("favicon.ico").await {
                let mut buf = Vec::new();
                if let Ok(_) = file.read_to_end(&mut buf).await {
                    return Ok(Response::builder().body(buf.into())?);
                }
            }
            Ok(not_found())
        }
        (&Method::GET, &["hello", x]) => {
            let hello = HelloTemplate {
                name: x,
                click_count: &state.click_count,
                loop_count: &state.loop_count,
            };
            let template = hello.render()?;
            Ok(Response::builder()
                .header("content-language", "en-US")
                .header("content-type", "text/html; charset=utf-8")
                .status(StatusCode::OK)
                .body(Body::from(template))
                .unwrap())
        }
        (&Method::GET, &["hello", rest]) => Ok(not_found()),
        _ => Ok(not_found()),
    }
    //Ok(Response::builder().body(Body::from("im body")).unwrap())
}

fn not_found() -> Response<Body> {
    let body = Body::from("Not Found");
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(body)
        .unwrap()
}

fn respond_with_html() -> http::response::Builder {
    Response::builder()
        .header("content-language", "en-US")
        .header("content-type", "text/html; charset=utf-8")
        .status(StatusCode::OK)
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
