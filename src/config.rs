use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, process::exit, fs};

static CONFIG: &str = "config.json";

pub type TargetMap = HashMap<String, Target>;

pub fn get_config() -> Config {
  if !Path::new(CONFIG).is_file() {
    println!(
      r#"
Need config.json.
Create config and restart.
Example.
{{
  "port":{{

  }},
  "proxy":{{
    "proxy.domain.com": 8080,
    "static.domain.com": "C:/"
  }}
}}
"#
    );
    exit(0)
  }

  let json: &[u8] = &fs::read(CONFIG).unwrap();
  let config: Config = serde_json::from_slice(json).unwrap();

  println!("#= Config Map ============================================");
  config.proxy.iter().for_each(|(domain, target)| {
    let (mode,target) = target.mode();
    println!("|{}| {} -> {}",mode , domain, target)
  });
  println!("#=========================================================");

  config
}

#[derive(Serialize, Deserialize,Debug)]
pub struct Config {
  pub port: u16,
  pub proxy: TargetMap,
}

#[derive(Serialize, Deserialize,Debug)]
#[serde(untagged)]
pub enum Target {
  Port(u16),
  Path(String),
}
impl Target {
  fn mode(&self) ->(&'static str,String) {
    match self {
      Target::Path(target) => ("Static",target.clone()),
      Target::Port(target) => ("Proxy ",format!("localhost:{}",target)),
    }
  }
  pub fn get(&self) -> (String,bool) {
    match self {
      Target::Path(target) => (target.clone(),false),
      Target::Port(target) => (format!("http://192.168.0.201:{}",target),true),
    }
  }
}