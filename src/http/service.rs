/// my local http service
use crate::http::helpers;
use crate::result::Result;
use crate::state::{Event, EventTx, State};

use std::collections::HashMap;

use std::sync::Arc;

// https://github.com/djc/askama
use askama::Template;
use log::{debug, error};

//use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response};
use tokio::sync::Mutex;

use url::form_urlencoded;

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
    click_count: &'a u32,
    loop_count: &'a u32,
    picture_count: usize,
    last_tag: u32,
    last_distance: u16,
}

#[derive(Template)]
#[template(path = "picture.html")]
struct PictureTemplate {
    image_id: usize,
    total_images: usize,
}

// The webserver task
pub async fn http_response(
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
                picture_count: state.pictures.len(),
                last_tag: state.last_tag_read().unwrap_or(0),
                last_distance: state.distance,
            };
            let template = hello.render()?;
            helpers::render_template(template)
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
            helpers::redirect_to("/".to_string())
        }
        (&Method::GET, &["favicon.ico"]) => helpers::get_png("cat-icon_64.png"),
        (&Method::POST, &["take_picture"]) => {
            let has_camera = { state.lock().await.has_camera };
            match has_camera {
                false => helpers::not_found(), // Should have a nice message about taking a picture or not
                true => {
                    // Send message to take picture
                    debug!("Requesting image be taken");
                    if let Err(err) = tx.send(Event::TakeImageRequest) {
                        error!("Error taking picture: {}", err);
                    };
                    helpers::redirect_to("/".to_string())
                }
            }
        }
        (&Method::GET, &["picture", numb]) => match numb.parse::<usize>() {
            Ok(image_id) => {
                let picts = &state.lock().await.pictures;
                if picts.len() > 0 && image_id < picts.len() {
                    let picture_template = PictureTemplate {
                        image_id,
                        total_images: picts.len(),
                    };
                    let template = picture_template.render()?;
                    helpers::render_template(template)
                } else {
                    helpers::not_found()
                }
            }
            Err(_e) => helpers::not_found(),
        },
        (&Method::GET, &["images", image_id]) => {
            debug!("Requesting image {}", image_id);
            match image_id.parse::<usize>() {
                Ok(image_id) => {
                    let picts = &state.lock().await.pictures;
                    if picts.len() > 0 && image_id < picts.len() {
                        let some_pict = &picts[image_id];
                        helpers::get_camera_image(some_pict.to_vec())
                    } else {
                        helpers::not_found()
                    }
                }
                Err(_e) => helpers::not_found(),
            }
        }
        (&Method::POST, &["delete_image", image_id]) => match image_id.parse::<usize>() {
            Ok(image_id) => {
                let picts = &state.lock().await.pictures;
                if picts.len() > 0 && image_id < picts.len() {
                    if let Err(err) = tx.send(Event::DeleteImage(image_id)) {
                        error!("Error deleting picture: {}", err);
                    };
                    helpers::redirect_to("/".to_string())
                } else {
                    helpers::unprocessable_entry()
                }
            }
            Err(_e) => helpers::unprocessable_entry(),
        },

        _ => {
            debug!("Not Found");
            helpers::not_found()
        }
    }
}
