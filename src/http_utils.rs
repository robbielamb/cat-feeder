use crate::result::Result;
use hyper::{Body, Response, StatusCode};

pub fn render_template(template: String) -> Result<Response<Body>> {
    let body = Body::from(template);
    Ok(Response::builder()
        .header("content-language", "en-US")
        .header("content-type", "text/html;charset=utf-8")
        .header("Cache-Control", "no-cache")
        .status(StatusCode::OK)
        .body(body)?)
}

pub fn redirect_to(location: String) -> Result<Response<Body>> {
    let foo = format!("Navigate to: {}", location);
    let body = Body::from(foo);
    Ok(Response::builder()
        .header("content-language", "en-US")
        .header("content-type", "text/html;charset=utf-8")
        .header("Cache-Control", "no-cache")
        .header("location", location)
        .status(StatusCode::FOUND)
        .body(body)?)
}

pub fn not_found() -> Result<Response<Body>> {
    let body = Body::from("Not Found");
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(body)?)
}
