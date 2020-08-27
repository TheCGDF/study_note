use serde::{Deserialize, Serialize};
use std::{env, process, fs};
use std::fs::File;
use std::io::Write;
use lazy_static::lazy_static;
use std::path::PathBuf;
use std::collections::{HashSet, HashMap};
use std::sync::Mutex;
use chrono::{DateTime, Utc};

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Config {
    pub token: String,
    pub group: i64,
    pub locks: HashSet<i64>,
    //<group id, user>
    pub notes: Vec<(i64, i64)>,
    //<group id, user, keywords>
    pub answers: Vec<(i64, i64, HashSet<String>)>,
    //<group id, noisy, last>
    pub chats: HashMap<i64, (bool, DateTime<Utc>)>,
}

lazy_static! {
    static ref CONFIG_PATH: PathBuf = env::current_exe().unwrap().parent().unwrap().join("config");
    pub static ref CONFIG:Mutex<Config>= Mutex::new(load());
}

pub fn load() -> Config {
    if !CONFIG_PATH.exists() {
        let mut config_file = File::create(&*CONFIG_PATH).unwrap();
        config_file.write_all(
            serde_json::to_string_pretty(&Config { ..Default::default() }).unwrap().as_bytes()
        ).unwrap();
        println!("config created");
        process::exit(0);
    }
    let config_content = fs::read_to_string(&*CONFIG_PATH).unwrap();
    return serde_json::from_str::<Config>(&config_content).unwrap();
}

impl Config {
    pub fn save(&self) {
        let mut config_file = File::create(&*CONFIG_PATH).unwrap();
        config_file.write_all(
            serde_json::to_string_pretty(self).unwrap().as_bytes()
        ).unwrap();
    }
}