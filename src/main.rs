mod database;
mod resp;

use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, Mutex},
};

use bytes::{Buf, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    stream::StreamExt,
    sync::oneshot,
    time::{delay_for, Duration},
};

use database::{Db, DbEntry};
use resp::{RespError, RespValue};

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
            "SET" => handle_set_command(data, db),
            "GET" => {
                let key = data.pop().unwrap().inner_string();
                if let Some(value) = db.lock().unwrap().get(&key) {
                    RespValue::new_simple(value.deref())
                } else {
                    RespValue::Null
                }
            }
            x => unimplemented!("{}", x),
        }
        .raw_bytes();

        while response.has_remaining() {
            stream.write_buf(&mut response).await.unwrap();
        }
    } else {
        panic!("Sometgin went wrong");
    }
}

fn handle_set_command(mut args: Vec<RespValue>, db: &Db) -> RespValue {
    let key = args.pop().unwrap().inner_string();
    let value = args.pop().unwrap().inner_string();

    let cancel_timeout = |previous: Option<DbEntry>| {
        if let Some(entry) = previous {
            if let Some(tx) = entry.timeout_channel {
                let _ = tx.send(());
            }
        }
    };

    if let Some(arg) = args.pop() {
        let mut arg = arg.inner_string();
        arg.make_ascii_uppercase();
        match arg.as_str() {
            "PX" => {
                let (tx, mut rx) = oneshot::channel();
                let expire = args.pop().unwrap().inner_string().parse().unwrap();
                let db_timeout = db.clone();

                let previous = {
                    db.lock()
                        .unwrap()
                        .insert(key.clone(), DbEntry::with_timeout(value, tx))
                };

                cancel_timeout(previous);

                tokio::spawn(async move {
                    delay_for(Duration::from_millis(expire)).await;
                    if rx.try_recv().is_ok() {
                        return;
                    }
                    db_timeout.lock().unwrap().remove(&key);
                });
            }
            x => unimplemented!("{}", x),
        }
    } else {
        let previous = db.lock().unwrap().insert(key, DbEntry::new(value));
        cancel_timeout(previous);
    }

    RespValue::new_simple("OK")
}
