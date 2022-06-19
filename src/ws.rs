use std::sync::Arc;

use hyper_tungstenite::{HyperWebsocket, tungstenite::Message};
use futures::{StreamExt, SinkExt, pin_mut, future::select};
use tokio::sync::Mutex;
use websocket_lite::Opcode;
use websocket_lite;

use crate::Req;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub async fn server(request: Req,client_websocket: HyperWebsocket, domain: String) -> Result<(), Error> {
  let client_websocket = client_websocket.await?;

  let url = format!("ws://{}{}",domain,request.uri());
  let mut server_websocket = websocket_lite::ClientBuilder::new(&url).unwrap();

  for (name,value) in request.headers() {
    server_websocket.add_header(
      name.to_string(),
      value.to_str().unwrap_or_default().to_string()
    )
  }

  let server_websocket = server_websocket.async_connect().await.unwrap();

  let (swrite,sread) = server_websocket.split();
  let (cwrite,cread) = client_websocket.split();

  let swrite = Arc::new(Mutex::new(swrite));
  let cwrite = Arc::new(Mutex::new(cwrite));
  
  let server_to_client = sread.for_each(|data| async {
    let cw = Arc::clone(&cwrite);
    match data {
      Ok(data)=>{
        let data = tokio_msg(data);
        cw.lock().await.send(data).await.unwrap();
      },
      Err(_)=>{
        let sw = Arc::clone(&swrite);
        sw.lock().await.send(websocket_lite::Message::close(None)).await.unwrap();
        cw.lock().await.close().await.unwrap();
      },
    }
  });

  let client_to_server = cread.for_each(|data| async {
    let sw = Arc::clone(&swrite);
    match data {
      Ok(data)=>{
        let data = lite_msg(data);
        sw.lock().await.send(data).await.unwrap();
      },
      Err(_)=>{
        let cw = Arc::clone(&cwrite);
        sw.lock().await.send(websocket_lite::Message::close(None)).await.unwrap();
        cw.lock().await.close().await.unwrap();
      },
    }
  });
  
  pin_mut!(server_to_client);
  pin_mut!(client_to_server);
  
  select(server_to_client,client_to_server).await;
  Ok(())
}

fn tokio_msg(from:websocket_lite::Message) -> Message { 
  match from.opcode() {
    Opcode::Text => Message::Text(from.as_text().unwrap().to_string()),
    Opcode::Binary => Message::Binary(from.data().to_vec()),
    Opcode::Ping => Message::Ping(from.data().to_vec()),
    Opcode::Pong => Message::Pong(from.data().to_vec()),
    Opcode::Close => Message::Close(None),
  }
}

fn lite_msg(from:Message) -> websocket_lite::Message {
  match from {
    Message::Text(data) => websocket_lite::Message::text(data),
    Message::Binary(data) => websocket_lite::Message::binary(data),
    Message::Ping(data) => websocket_lite::Message::ping(data),
    Message::Pong(data) => websocket_lite::Message::pong(data),
    Message::Close(_) => websocket_lite::Message::close(None),
    Message::Frame(_) => unreachable!(),
  }
}