/// https://github.com/djc/askama
/// and hyper.rs
use askama::Template;

use tokio::task;
use tokio::time;
use tokio::time::delay_for;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;

use futures_util::future::join;

use std::time::Duration;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Result, Server, StatusCode};

static NOTFOUND: &[u8] = b"Not Found";

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name:  &'a str,
    clickCount: &'a u32,
    loopCount: &'a u32,

}

struct Shared {
    clickCount: u32,
    loopCount: u32,
}

impl Shared {
    fn new() -> Self {
        Shared{ clickCount: 0, loopCount: 0 }
    }
}

#[tokio::main]
async fn main() {
    //pretty_env_logger::init();

    let addr = "127.0.0.1:1337".parse().unwrap();

    // For the most basic of state, we just share a counter, that increments
    // with each request, and we send its value back in the response.
    let counter = Arc::new(AtomicUsize::new(0));

    let state = Arc::new(Mutex::new(Shared::new()));

    let cloneState = Arc::clone(&state);
    let make_service = make_service_fn( move |_| {
        //let counter = counter.clone();
        
        let count_state = Arc::clone(&cloneState);
        async move {
            let state = Arc::clone(&count_state);
            Ok::<_, hyper::Error>(service_fn( move |request: Request<Body>|  {
                //let count = counter.fetch_add(1, Ordering::AcqRel);
                
                let state = Arc::clone(&state);
            
              
                     response_function(request, state)
             

                
            }))
            
        }
    });

    let server = Server::bind(&addr).serve(make_service);

    let my_task = task::spawn(async move {
        let state = Arc::clone(&state);
        loop {
            {
              let mut state = state.lock().await;
              state.loopCount += 1;
              println!("In the spawned loop {} times", state.loopCount);            
            }
            delay_for(Duration::from_secs(5)).await;
        }
    });

    println!("Starting Server");

   /*  let result = join.await;
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    } */

    let _ret = join(my_task, server).await;
}

async fn response_function(req: Request<Body>, state: Arc<Mutex<Shared>>) -> Result<Response<Body>> {
    let state = Arc::clone(&state);
    let mut state = state.lock().await;
    state.clickCount += 1;

    

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            let hello = HelloTemplate {
                name: "hey there",
                clickCount: &state.clickCount,
                loopCount: &state.loopCount,
            };
            let template = hello.render().unwrap();
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(template))
                .unwrap())
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
