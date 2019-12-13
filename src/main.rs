/// https://github.com/djc/askama
/// and hyper.rs
use askama::Template;

use tokio::task;
//use tokio::time;
/* use tokio::fs::File;
use tokio::io::AsyncReadExt; */
use tokio::sync::{mpsc, Mutex};
use tokio::time::delay_for;

use futures_util::future::join3;

use std::time::Duration;

use std::sync::{
    //atomic::{AtomicUsize, Ordering},
    Arc,
};

//se hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Result, Server, StatusCode};


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
    IncLoop
}

/// Shorthand for the transmit half of the message channel.
type Tx = mpsc::UnboundedSender<Event>;

/// Shorthand for the receive half of the message channel.
type Rx = mpsc::UnboundedReceiver<Event>;


#[tokio::main]
async fn main() {
    //pretty_env_logger::init();

    let addr = "0.0.0.0:1337".parse().unwrap();

    let (mut tx, mut rx) = mpsc::unbounded_channel::<Event>();

    let tx = Arc::new(Mutex::new(tx));

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
                response_function(request, state, tx)
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
                    println!("Error sending message");
                }
                let state = state.lock().await;
                //state.loop_count += 1;
                println!("In the spawned loop {} times", state.loop_count);
            }
            delay_for(Duration::from_secs(5)).await;
        }
    });

    println!("Starting Server");

    /*  let result = join.await;
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    } */

    let _ret = join3(reducer_task, my_task, server).await;
}

async fn response_function(
    req: Request<Body>,
    state: Arc<Mutex<Shared>>,
    tx: Arc<Mutex<Tx>>
) -> Result<Response<Body>> {
    //let state = Arc::clone(&state);
    if let Err(_err) = tx.lock().await.send(Event::IncClick) {
        println!("Error sending message");
    }

    let state = state.lock().await;
    //state.click_count += 1;

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            let hello = HelloTemplate {
                name: "hey there",
                click_count: &state.click_count,
                loop_count: &state.loop_count,
            };
            let template = hello.render().unwrap();
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(template))
                .unwrap())
        }
        (&Method::POST, "/take_picture") => {
            let body = req.body();
            
            Ok(Response::builder().status(StatusCode::MOVED_PERMANENTLY)
        .header("Location", "/get_image/1").body(Body::from("Moved")).unwrap())
        }
        _ => Ok(not_found()),
    }
}

fn not_found() -> Response<Body> {
    let body = Body::from("Not Found");
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(body)
        .unwrap()
}


async fn reducer(event: Event, state: &Mutex<Shared>)  {
    match event {
        Event::IncLoop => {
            state.lock().await.loop_count += 1;
        }
        Event::IncClick => {
            state.lock().await.click_count += 1;
        }
    };
    
}