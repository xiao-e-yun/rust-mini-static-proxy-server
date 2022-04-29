use serde::{Deserialize,Serialize};
use std::{path::Path, collections::BTreeMap, sync::Arc, process::exit};
use warp::{Filter, host::Authority, reject};
use tokio::fs;


pub mod http;
pub mod ws;


static CONFIG: &str = "config.json";
static CERT: &str = "cert.pem";
static KEY: &str = "key.pem";



#[tokio::main]
async fn main() {
  println!("read config");
  let targets= get_config().await;
  let target_http = Arc::new(targets.0);
  let target_ws = Arc::new(targets.1);

  println!("setting filter");
  let filter = warp::ws()
    .and(warp::host::optional())
    .and(warp::path::full())
    .and_then(move|ws,host,path|{
      let targets = Arc::clone(&target_ws);
      async move{

        let target = choose(targets.to_vec(),host);
        match target {
          Some(target) => Ok(ws::ws(target,ws,path).await),
          None => Err(reject::not_found()),
        }
  
      }
    })
    .or(
      warp::host::optional()
      .and(warp::header::headers_cloned())
      .and(warp::path::full())
      .and(warp::method())
      .and(warp::body::bytes())
      .and_then(move|host,headers,path,method,body|{
        let targets = Arc::clone(&target_http);
        async move {
        
          let target = choose(Arc::clone(&targets).to_vec(),host);
          match target {
            Some(target) => Ok(http::http(target,headers,path,method,body).await),
            None => Err(reject::not_found()),
          }

        }
      }
      ));

  println!("create server");
  let server = warp::serve(filter).tls().cert_path(CERT).key_path(KEY);

  println!("running");
  println!("binding port 8000");
  server.run(([127, 0, 0, 1], 8000)).await;

  println!("stopped");
}

#[derive(Clone)]
pub enum Target {
  Proxy(String,Port),
  Static(String,String)
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum TargetValue {
  Port(Port),
  Path(String),
}

type Port = u16;

fn choose(targets: Vec<Target> ,host: Option<Authority>)-> Option<TargetValue> {
  let host = host.unwrap().host().to_string();
  for target in targets.into_iter() {
    match target {
      Target::Proxy(name,val) => {
        if host == name { return Some(TargetValue::Port(val)); }
      },
      Target::Static(name,val) => {
        if host == name { return Some(TargetValue::Path(val)); }
      }
    }
  }

  None
}

async fn get_config()->(Vec<Target>,Vec<Target>) {

  if !Path::new(CONFIG).is_file() {
    
    
    println!(r#"
Need config.json.
Create config and restart.
Example.
{{
  "proxy.domain.com": 8080,
  "static.domain.com": "C:/"
}}
"#);
  exit(0)
  }

  let json: &[u8] = &fs::read(CONFIG).await.unwrap()[..];
  let config: BTreeMap<String,TargetValue> = serde_json::from_slice(json).unwrap();

  let mut http = vec![];
  let mut ws = vec![];

  let mut view = vec![];

  println!("#=Config Map==========================");

  config.into_iter().for_each(|tar|{
    let domain = tar.0;

    let target = match tar.1 {
      TargetValue::Port(p) => {
        view.push(format!("Proxy | {} -> localhost:{}",domain,&p));
        let target = Target::Proxy(domain, p);
        ws.push(target.clone());
        target
      },
      TargetValue::Path(p) => {
        view.push(format!("Static| {} -> {}",domain,&p));
        Target::Static(domain, p)
      }
    };

    http.push(target)
  });

  view.sort_by(|a,b|b.len().cmp(&a.len()));
  println!("|{}",view.join("\n|"));

  println!("#=====================================");

  (http,ws)
}