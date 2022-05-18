mod resp;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bytes::{Buf, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    stream::StreamExt,
};

use resp::{RespError, RespValue};

type Db = Arc<Mutex<HashMap<String, String>>>;

#[tokio::main]
async fn main() {
    let mut listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    let mut incoming = listener.incoming();

    let db = Arc::new(Mutex::new(HashMap::new()));

    while let Some(stream) = incoming.next().await {
        let stream = stream.unwrap();
        let db = db.clone();
        tokio::spawn(async move { handle_connection(stream, db).await });
    }
}

async fn handle_connection(mut stream: TcpStream, db: Db) {
    loop {
        let mut data = BytesMut::new();
        let val = loop {
            stream.read_buf(&mut data).await.unwrap();

            match RespValue::try_from_bytes(&mut data) {
                Ok(r) => break r,
                Err(e) => {
                    if !matches!(e, RespError::Incomplete) {
                        panic!("Somethin went wrong");
                    }
                }
            }
        };

        handle_command(val, &mut stream, &db).await;
    }
}

async fn handle_command(val: RespValue, stream: &mut TcpStream, db: &Db) {
    if let RespValue::Array(mut data) = val {
        data.reverse();
        let command = match data.pop().unwrap() {
            RespValue::Bulk(mut cmd) => {
                cmd.make_ascii_uppercase();
                cmd
            }
            _ => unreachable!(),
        };

        let mut response = match command.as_str() {
            "PING" => RespValue::new_simple("PONG"),
            "ECHO" => data.pop().unwrap(),
            "SET" => {
                let key = data.pop().unwrap().inner_string();
                let value = data.pop().unwrap().inner_string();

                {
                    db.lock().unwrap().insert(key, value);
                }

                RespValue::new_simple("OK")
            }
            "GET" => {
                let key = data.pop().unwrap().inner_string();
                if let Some(value) = db.lock().unwrap().get(&key) {
                    RespValue::new_simple(value)
                } else {
                    RespValue::Null
                }
            }
            x => todo!("{}", x),
        }
        .raw_bytes();

        while response.has_remaining() {
            stream.write_buf(&mut response).await.unwrap();
        }
    } else {
        panic!("Sometgin went wrong");
    }
}
