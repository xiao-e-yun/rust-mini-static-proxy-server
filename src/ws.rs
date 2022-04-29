use futures::{SinkExt, pin_mut, future::select};
use std::{process::exit, sync::Arc};

use tokio_tungstenite::tungstenite::Message as TokioTunMsg;
use futures::StreamExt;
use tokio::sync::Mutex;
use warp::{
  path::FullPath,
  ws::Ws,
  Reply,
};

use crate::TargetValue;

pub async fn ws(
  target: TargetValue,
  ws: Ws,
  path: FullPath,
) -> Result<impl Reply, warp::http::Error> {
  let port = match target {
    TargetValue::Port(port) => port,
    TargetValue::Path(_) => exit(0),
  };

  Ok(ws.on_upgrade(move |ws| async move {
    let url = format!("ws://127.0.0.1:{}{}", port, path.as_str());
    println!("[proxy-ws] {}",url);
    let stream = match tokio_tungstenite::connect_async(url).await {
      Ok((stream,_)) => stream,
      Err(_) => return 
    };

    let (cs, cr) = ws.split();
    let (ss, sr) = stream.split();

    let cs = Arc::new(Mutex::new(cs));
    let ss = Arc::new(Mutex::new(ss));

    let input = cr.for_each(|v| async {
      let data = tokio_msg(v.unwrap_or(warp::ws::Message::close()));
      if let Some(data) = data { Arc::clone(&ss).lock().await.send(data).await.unwrap(); }
    });

    let output = sr.for_each(|v| async {
      let data = warp_msg(v.unwrap_or(TokioTunMsg::Close(None)));
      if let Some(data) = data { Arc::clone(&cs).lock().await.send(data).await.unwrap(); }
    });
    
    pin_mut!(input);
    pin_mut!(output);
    
    select(input,output).await;
  }))
}

fn tokio_msg(from:warp::ws::Message) -> Option<TokioTunMsg> {  
  if from.is_ping() { Some(TokioTunMsg::Ping(from.into_bytes())) }
  else if from.is_pong() { Some(TokioTunMsg::Pong(from.into_bytes())) }
  else if from.is_binary() { Some(TokioTunMsg::Binary(from.into_bytes())) }
  else if from.is_text() { Some(TokioTunMsg::Text(from.to_str().unwrap().to_string())) }
  else if from.is_close() { Some(TokioTunMsg::Close(None)) }
  else { None }
}
fn warp_msg(from:TokioTunMsg) -> Option<warp::ws::Message> {
  match from {
    TokioTunMsg::Text(data) => Some(warp::ws::Message::text(data)),
    TokioTunMsg::Binary(data) => Some(warp::ws::Message::binary(data)),
    TokioTunMsg::Ping(data) => Some(warp::ws::Message::ping(data)),
    TokioTunMsg::Pong(data) => Some(warp::ws::Message::pong(data)),
    _ => None,
  }
}