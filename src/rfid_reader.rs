use std::{env, io, str};

use tokio::task;
use tokio_util::codec::{Decoder, Encoder};
use futures::stream::StreamExt;

use bytes::BytesMut;

const DEFAULT_TTY: &str = "/dev/ttyUSB0";

pub fn rfid_reader() -> task::JoinHandle<()> {
    task::spawn(async move {
        let settings = tokio_serial::SerialPortSettings::default();
     
        let mut port = tokio_serial::Serial::from_path(tty_path, &settings).unwrap();
    })
}