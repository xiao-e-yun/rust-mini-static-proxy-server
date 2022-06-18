use hyper_tungstenite::{HyperWebsocket, tungstenite::Message};
use futures::{StreamExt, SinkExt};

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub async fn server(websocket: HyperWebsocket, domain: String) -> Result<(), Error> {
  let mut websocket = websocket.await?;
  while let Some(message) = websocket.next().await {
      match message? {
          Message::Text(msg) => {
              println!("Received text message: {}", msg);
              websocket.send(Message::text("Thank you, come again.")).await?;
          },
          Message::Binary(msg) => {
              println!("Received binary message: {:02X?}", msg);
              websocket.send(Message::binary(b"Thank you, come again.".to_vec())).await?;
          },
          Message::Ping(msg) => {
              // No need to send a reply: tungstenite takes care of this for you.
              println!("Received ping message: {:02X?}", msg);
          },
          Message::Pong(msg) => {
              println!("Received pong message: {:02X?}", msg);
          }
          Message::Close(msg) => {
              // No need to send a reply: tungstenite takes care of this for you.
              if let Some(msg) = &msg {
                  println!("Received close message with code {} and message: {}", msg.code, msg.reason);
              } else {
                  println!("Received close message");
              }
          },
          Message::Frame(msg) => {
             unreachable!();
          }
      }
  }

  Ok(())
}
