use std::{io, str};

use log::{debug, error, info, trace};

use futures::{
    future::FutureExt, // for `.fuse()`
  
    select,
    stream::{StreamExt},
};

use tokio::sync::watch;
use tokio::task;
use tokio_util::codec::{Decoder, Encoder};

use bytes::buf::Buf;
use bytes::BytesMut;

use crate::state::{Event, RunState, Tx};

const DEFAULT_TTY: &str = "/dev/ttyS0";

struct RFIDCodec;

impl Decoder for RFIDCodec {
    type Item = u32;
    type Error = io::Error;

    ///
    /// |02| 10 bytes | 2 byte checksum |03| => 14 bytes
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Read until we hit a 2.
        // Return eat the bytes up to the 2 and error if there is data before a 2
        // If the first byte is a 2, read upto 14 bytes.
        // If there are fewer then 14 bytes return Ok(None)
        // Consume these 14 bytes.
        // Make sure the last one is a 3 and return error if it's not
        // Validate the check sum and return an error if it's not
        // Return bytes 1..+11 as the rfid number

        let test_src = src.as_ref();
        // Need at least 14 bytes to do work
        if test_src.len() < 14 {
            return Ok(None);
        }

        let start_pos = find_byte(test_src, 2);

        if let Some(0) = start_pos {
            // The last byte should be a 3. If not we have garbage on the line
            if test_src[13] != 3 {
                // Consume the first byte then upto the next byte of value 2.
                let _ = src.get_u8();
                return Err(consume_bytes(src, find_byte(src.as_ref(), 2)));
            }

            // We have a valid frame. Eat the frame.
            let line = src.split_to(14);
            let my_slice: &[u8] = line.as_ref();

            let msg_data: &[u8] = &my_slice[1..11];
            let _version_data: &[u8] = &my_slice[1..3];
            let tag_data: &[u8] = &my_slice[3..11];
            let checksum_data: &[u8] = &my_slice[11..13];

            //let checksum= str::from_utf8(&my_slice[11..13]).map(|x| u16::from_str_radix(x, 16)).unwrap().unwrap();

            let checksum_str = checksum_data
                .into_iter()
                .map(|c| char::from(*c))
                .collect::<String>();

            let checksum = u16::from_str_radix(&checksum_str, 16).unwrap_or(0);

            trace!(
                "checksum: {}, checksum3: {}{}, slice: {:x?}",
                checksum,
                char::from(checksum_data[0]),
                char::from(checksum_data[1]),
                my_slice
            );

            let comupted_checksum = compute_checksum(msg_data).unwrap();
            trace!(
                "Computed checksum {}, HEX: {:X?}",
                comupted_checksum,
                comupted_checksum
            );

            let tag_str = tag_data
                .into_iter()
                .map(|c| char::from(*c))
                .collect::<String>();

            let tag = u32::from_str_radix(&tag_str, 16).unwrap_or(0);
            trace!("Tag: {:?} tag_str: {:?}", tag, tag_str);

            return Ok(Some(tag));

        /* return match str::from_utf8(&my_slice[3..11]) {
            Ok(s) => Ok(Some(s.to_string())),

            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Invalid String")),
        }; */
        } else {
            // The first byte wasn't a 2. Consume the buffer upto the 2 we found
            return Err(consume_bytes(src, start_pos));
        }
    }
}

fn compute_checksum(s: &[u8]) -> Option<u16> {
    if s.len() != 10 {
        return None;
    }

    let mut acc: u16 = 0;
    for i in 0..5 {
        let i = i * 2;
        let value = &s[i..(i + 2)];
        if let Ok(number_string) = str::from_utf8(value) {
            acc = acc ^ u16::from_str_radix(number_string, 16).unwrap_or(0)
        }
    }
    Some(acc)
}

fn find_byte(buf: &[u8], byte: u8) -> Option<usize> {
    buf.as_ref().iter().position(|b| *b == byte)
}

fn consume_bytes(src: &mut BytesMut, count: Option<usize>) -> io::Error {
    match count {
        Some(n) => {
            let garbage: BytesMut = src.split_to(n);

            io::Error::new(
                io::ErrorKind::Other,
                format!("Garbage before start byte {:?}", garbage),
            )
        } // Consume up to X and return an error
        None => {
            src.clear();
            io::Error::new(io::ErrorKind::Other, "All bytes in buffer are bad")
        }
    }
}

impl Encoder for RFIDCodec {
    type Item = String;
    type Error = io::Error;

    fn encode(&mut self, _item: Self::Item, _dst: &mut BytesMut) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub fn rfid_reader(tx: Tx, mut stop_rx: watch::Receiver<RunState>) -> task::JoinHandle<()> {
    task::spawn(async move {
        debug!("starting rfid reader");
        // Default settings look to be okay
        let mut settings = tokio_serial::SerialPortSettings::default();

        //settings.timeout = std::time::Duration::from_secs(190);

        let mut port = tokio_serial::Serial::from_path(DEFAULT_TTY, &settings).unwrap();

        port.set_exclusive(false)
            .expect("Unable to set serial port exclusive to false");

        let mut reader = RFIDCodec.framed(port);
        //pin_mut!(reader);
        loop {
            /* match reader.next().await {
            Some(line_result) => {
                let line = line_result.expect("Failed to read line");
                info!("{}", line)
            } */

            select! {
                some_id = reader.next().fuse() => {
                    match some_id {
                        Some(line) => {
                            let line = line.expect("Failet to read");
                            if let Err(err) = tx.send(Event::ReadTag(line)) {
                                error!("Error updating last read tag: {}", err);
                            }
                    info!("{}", line)
                        }
                        None => ()
                    }
                }
                event = stop_rx.recv().fuse() => if let Some(RunState::Shutdown) = event {
                    debug!("Ending RFID task");
                    break
                }
            }
        }

        /*  while let Some(line_result) = get_next(&mut reader).await{
            let line = line_result.expect("Failed to read line");
            if let Err(err) = tx.send(Event::ReadTag(line)) {
                error!("Error updating last read tag: {}", err);
            }
            info!("{}", line);
        } */

        debug!("exiting");
    })
}

/* async fn get_next(reader: &mut Framed<tokio_serial::Serial, RFIDCodec>) -> Option<Result<u32, std::io::Error>> {
    reader.next().await
}
 */
