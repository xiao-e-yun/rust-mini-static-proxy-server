// use std::path::PathBuf;

// use new_mime_guess::Mime;
// use reqwest::{Method, StatusCode, header::HeaderValue};
// use tokio::fs;
// use urlencoding::decode;
// use warp::{
//   hyper::body::Bytes,
//   hyper::{HeaderMap, Response},
//   path::FullPath, Reply,
// };

use hyper_staticfile::ResolveResult;

use crate::{Req,Res};

pub async fn server(mut request: Req, domain: String) -> Res {
  *request.uri_mut() = format!("{}{}",&domain,request.uri()).parse().unwrap();
  hyper::Client::new().request(request).await.unwrap_or(
    hyper::Response::builder()
      .status(503)
      .body(hyper::Body::from("503 Service Unavailable"))
      .unwrap()
  )
}

pub async fn static_server(request: Req, domain: String) -> Res {
  let resolve = hyper_staticfile::resolve(domain, &request).await.unwrap_or(ResolveResult::PermissionDenied);
  match resolve {
    ResolveResult::Found(file, metadata, mime) => {
      hyper_staticfile::FileResponseBuilder::new()
        .build(file, metadata, mime.to_string())
        .unwrap()
    }
    _ => hyper::Response::builder()
      .status(404)
      .body(hyper::Body::from("404 Not Found"))
      .unwrap()
  }
}