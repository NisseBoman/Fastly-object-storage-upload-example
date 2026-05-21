mod config;
mod fos;
mod html;

use config::FOS;
use fastly::http::{Method, StatusCode};
use fastly::{Error, Request, Response};

fn is_upload_request(req: &Request) -> bool {
    let method = req.get_method();
    if *method != Method::PUT && *method != Method::POST {
        return false;
    }

    req.get_query_parameter("key").is_some()
        || req.get_path().starts_with("/api/upload")
}

#[fastly::main]
fn main(req: Request) -> Result<Response, Error> {
    let method = req.get_method().clone();
    let path = req.get_path().to_string();

    if method == Method::GET && (path == "/" || path == "/index.html") {
        return Ok(html::index_page(&req));
    }

    if is_upload_request(&req) {
        return fos::upload_to_fos(&FOS, req);
    }

    if method == Method::GET {
        return Ok(Response::from_status(StatusCode::NOT_FOUND)
            .with_body_text_plain("Not found. Open / for the upload demo."));
    }

    Ok(Response::from_status(StatusCode::METHOD_NOT_ALLOWED).with_body_text_plain(
        "Supported: GET / (demo UI), POST|PUT /api/upload?key=<object-key>",
    ))
}
