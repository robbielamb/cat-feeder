use hyper::{Body, Response, StatusCode};
use log::error;

use crate::assets::Image;
use crate::result::Result;

// Render a string as the body to be returned
pub fn render_template(template: String) -> Result<Response<Body>> {
    let body = Body::from(template);
    Ok(Response::builder()
        .header("content-language", "en-US")
        .header("content-type", "text/html;charset=utf-8")
        .header("Cache-Control", "no-cache")
        .status(StatusCode::OK)
        .body(body)?)
}

/// Redirect to the given location. Location is lame and just a string right now
pub fn redirect_to(location: String) -> Result<Response<Body>> {
    let body = Body::from(format!("Navigate to: {}", location));
    Ok(Response::builder()
        .header("content-language", "en-US")
        .header("content-type", "text/html;charset=utf-8")
        .header("Cache-Control", "no-cache")
        .header("location", location)
        .status(StatusCode::FOUND)
        .body(body)?)
}

/// This has not been found. 404
pub fn not_found() -> Result<Response<Body>> {
    let body = Body::from("Not Found");
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(body)?)
}

pub fn unprocessable_entry() -> Result<Response<Body>> {
    let body = Body::from("Unprocessable entry");
    let res = Response::builder()
        .status(StatusCode::UNPROCESSABLE_ENTITY)
        .body(body)?;
    Ok(res)
}

pub fn get_png(name: &str) -> Result<Response<Body>> {
    if let Some(img) = Image::get(name) {
        let res = Response::builder()
            .header("Content-Type", "image/png")
            .body(Body::from(img))?;
        Ok(res)
    } else {
        error!("Image not found: {}", "name");
        not_found()
    }
}

pub fn get_ico(name: &str) -> Result<Response<Body>> {
    if let Some(img) = Image::get(name) {
        let res = Response::builder()
            .header("Content-Type", "image/x-icon")
            .body(Body::from(img))?;
        Ok(res)
    } else {
        error!("Image not found: {}", "name");
        not_found()
    }
}

pub fn get_camera_image(image: Vec<u8>) -> Result<Response<Body>> {
    let body = Body::from(image);

    Ok(Response::builder()
        .header("Content-Type", "image/jpg")
        .body(body)?)
}
