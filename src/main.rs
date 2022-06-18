use hyper::{server::conn::Http, service::service_fn, Body, Request, Response};
use tokio::{net::TcpListener, spawn};
use tokio_rustls::{TlsAcceptor, rustls::{PrivateKey, Certificate}};
use rustls_pemfile;

use std::{net::SocketAddr, sync::Arc};

use crate::{config::Config, router::handle};

#[macro_use]
extern crate lazy_static;

pub mod config;
pub mod http;
pub mod router;
pub mod ws;

pub type Req = Request<Body>;
pub type Res = Response<Body>;

lazy_static! {
  static ref CONFIG: Config = config::get_config();
}

static KEY: &str = "key.pem";
static CERT: &str = "cert.pem";

#[tokio::main]
async fn main() {
  let port = CONFIG.port;
  println!("Port {}", port);

  println!("Creating Server");
  let acceptor = tls().await;
  let listener = TcpListener::bind(SocketAddr::from(([0,0,0,0], port))).await.unwrap();

  println!("#= Runing ================================================");
  loop {
    let (tcp_stream, _) = listener.accept().await.unwrap();
    let acceptor = acceptor.clone();

    spawn(async move {
      let tls_stream = acceptor.accept(tcp_stream).await.unwrap();
      let server = Http::new()
        .http1_title_case_headers(true)
        .http1_preserve_header_case(true)
        .http2_enable_connect_protocol()
        .serve_connection(tls_stream, service_fn(handle));
      if let Err(e) = server.await {
        eprintln!("server error: {}", e);
      };
    });
  }
}

async fn tls() -> TlsAcceptor {

  let mut key: Vec<PrivateKey> = {
    let file = std::fs::File::open(KEY).unwrap();
    let mut buf = std::io::BufReader::new(file);
    rustls_pemfile::pkcs8_private_keys(&mut buf).map(|mut keys| keys.drain(..).map(PrivateKey).collect()).unwrap()
  };

  let cert: Vec<Certificate> = {
    let file = std::fs::File::open(CERT).unwrap();
    let mut buf = std::io::BufReader::new(file);
    rustls_pemfile::certs(&mut buf)
    .map(|mut certs| certs.drain(..).map(Certificate).collect()).unwrap()
  };

  let config = tokio_rustls::rustls::ServerConfig::builder()
    .with_safe_defaults()
    .with_no_client_auth()
    .with_single_cert(cert, key.remove(0))
    .unwrap();

  TlsAcceptor::from(Arc::new(config))
}
