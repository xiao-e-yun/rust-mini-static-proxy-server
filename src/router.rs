use hyper::{Response, Body};

use crate::{ws, http, Res, Req, CONFIG};
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub async fn handle(mut request:Req) -> Result<Res,Error> {
  let domain = request.headers().get("host").unwrap().to_str().unwrap();
  let target = CONFIG.proxy.get(&domain.to_string());

  
  match target {
    Some(target) => {  
      print!("{} -> ",domain);
      let (domain, proxy) = target.get();
      println!("{}",domain);

      let is_ws = hyper_tungstenite::is_upgrade_request(&request);

      if !proxy {
        if is_ws { return Ok(Response::new(Body::from("not allowed update to websocket"))); }

        return Ok(http::static_server(request,domain).await);
      }

      if is_ws {
          let (response, websocket) = hyper_tungstenite::upgrade(&mut request, None)?;
    
          // Spawn a task to handle the websocket connection.
          tokio::spawn(async move {
              if let Err(e) = ws::server(websocket,domain).await { eprintln!("[Error] Websocket connection: {}", e); }
          });
    
          // Return the response so the spawned future can continue.
          Ok(response)
      } else {
          // Handle regular HTTP requests here.
          Ok(http::server(request,domain).await)
      }
    },
    None => Ok(Response::new(Body::from("No Server")))
  }
}

