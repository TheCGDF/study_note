use serde::{Deserialize, Serialize};
use std::{env, process, fs};
use std::fs::{File, OpenOptions};
use std::io::Write;
use lazy_static::lazy_static;
use std::path::PathBuf;


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Config {
    pub token: String,
    pub group: i64,
    pub last: i64,
}

lazy_static! {
    static ref CONFIG_PATH: PathBuf = env::current_exe().unwrap().parent().unwrap().join("config");
}

pub fn load() -> Config {
    if !CONFIG_PATH.exists() {
        let mut config_file = File::create(&*CONFIG_PATH).unwrap();
        config_file.write_all(
            serde_json::to_string_pretty(&Config {
                token: String::new(),
                group: 0,
                last: 0,
            }).unwrap().as_bytes()
        ).unwrap();
        println!("config created");
        process::exit(0);
    } else {
        let config_content = fs::read_to_string(&*CONFIG_PATH).unwrap();
        let config: Config = serde_json::from_str(&config_content).unwrap();
        return config;
    }
}

impl Config {
    pub fn save(&self) {
        let mut config_file = OpenOptions::new().write(true).open(&*CONFIG_PATH).unwrap();
        config_file.write_all(
            serde_json::to_string_pretty(self).unwrap().as_bytes()
        ).unwrap();
    }
}