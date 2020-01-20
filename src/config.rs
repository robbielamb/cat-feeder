use serde_derive::{Deserialize, Serialize};
//use tokio::prelude::*;
//use tokio::fs::{File, read};
use toml;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub title: String,
    pub listen_port: String,
    pub port: Option<u16>,
    pub rfid: Rfid,
    pub distance: Distance,
}

#[derive(Deserialize, Serialize)]
pub struct Rfid {
    pub valid_ids: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Distance {
    pub near_value: u16,
    pub far_value: u16,
    pub alert_pin: u8,
    pub interval: u64,
}

pub fn read_config() -> Config {
    let lines = std::fs::read_to_string("cat-feeder.toml").expect("Config file not found");

    let config = toml::from_str(&lines);

    config.expect("Error parsing config file")
}
