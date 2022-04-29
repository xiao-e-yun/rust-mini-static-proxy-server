use std::path::PathBuf;

use new_mime_guess::Mime;
use reqwest::{Method, StatusCode, header::HeaderValue};
use tokio::fs;
use urlencoding::decode;
use warp::{
  hyper::body::Bytes,
  hyper::{HeaderMap, Response},
  path::FullPath, Reply,
};

use crate::{
  TargetValue,
  TargetValue::{Path as TargetPath, Port as TargetPort},
};

pub async fn http(
  target: TargetValue,
  headers: HeaderMap,
  query: FullPath,
  method: Method,
  body: Bytes,
) -> Result<impl Reply, warp::http::Error> {
  let mut out_headers = HeaderMap::default();
  let out_status;
  let out_body;
  match target {
    TargetPort(port) => {
      let client = reqwest::Client::new();
      let url = format!("http://127.0.0.1:{}{}", port, query.as_str());

      println!("[Proxy] {}",url);
      let res = client
        .request(method, url)
        .headers(headers)
        .body(body)
        .send()
        .await;
      match res {
        Ok(res) => {
          out_status = res.status();
          out_headers = res.headers().clone();
          out_body = res.bytes().await.unwrap().to_vec();
        }
        Err(_) => {
          out_status = StatusCode::BAD_GATEWAY;
          out_body = b"Bad Gateway".to_vec();
        },
      }

    }
    TargetPath(path) => {
      let mut path = PathBuf::from(&path);
      for n in decode(query.as_str()).unwrap().to_string().split("/") {
        if n.contains(":") { continue; }
        if n == ".." { continue; }
        path = path.join(n);
      }

      println!("[Static] {}",path.to_str().unwrap());
      
      (out_status, out_body) = match path.is_file() {
        true => {

          let (data, mime) = get_static(path).await;
          out_headers.insert("Content-Type",HeaderValue::from_str(&(mime.to_string())).unwrap());

          (
            StatusCode::OK,
            data,
          )
        }
        false => {
          let index = path.join("index.html");
          if path.is_dir() && index.is_file() {

            let (data, mime) = get_static(index).await;
            out_headers.insert("Content-Type",HeaderValue::from_str(&(mime.to_string())).unwrap());

            (
              StatusCode::OK,
              data,
            )
          } else {
            (
              StatusCode::NOT_FOUND,
              b"Not Found".to_vec(),
            )
          }
        }
      };
    
    }
  };

  {
    let mut out = Response::builder().status(out_status);

    let mut_header = out.headers_mut().unwrap();
    for header in out_headers {
      mut_header.insert(header.0.unwrap(), header.1);
    }

    out.body(out_body)
  }
}

async fn get_static(path: PathBuf)-> (Vec<u8>,Mime){
  let data = fs::read(&path).await.unwrap_or_default();
  let mime = new_mime_guess::from_path(path).first().unwrap_or(new_mime_guess::from_ext("txt").first().unwrap());
  (data,mime)
}