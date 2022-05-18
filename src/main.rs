mod resp;

use bytes::{Buf, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    stream::StreamExt,
};

use resp::{RespError, RespValue};

#[tokio::main]
async fn main() {
    let mut listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    let mut incoming = listener.incoming();

    while let Some(stream) = incoming.next().await {
        let stream = stream.unwrap();
        tokio::spawn(async move { handle_connection(stream).await });
    }
}

async fn handle_connection(mut stream: TcpStream) {
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

        handle_command(val, &mut stream).await;
    }
}

async fn handle_command(val: RespValue, stream: &mut TcpStream) {
    if let RespValue::Array(mut val) = val {
        val.reverse();
        let command = match val.pop().unwrap() {
            RespValue::Bulk(mut cmd) => {
                cmd.make_ascii_uppercase();
                cmd
            }
            _ => unreachable!(),
        };

        let mut response = match command.as_str() {
            "PING" => RespValue::new_simple("PONG"),
            "ECHO" => val.pop().unwrap(),
            x => todo!("{}", x),
        }
        .raw_bytes();

        dbg!(&response);

        // let mut response = RespValue::new_simple("PONG").raw_bytes();
        while response.has_remaining() {
            stream.write_buf(&mut response).await.unwrap();
        }
    } else {
        panic!("Sometgin went wrong");
    }
}
