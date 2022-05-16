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
    if let RespValue::Array(val) = val {
        let (command, _data) = val.split_at(1);
        let _command = &command[0];
        // match command[0] {

        // }
        let mut response = RespValue::Simple("PONG".to_string()).raw_bytes();
        while response.has_remaining() {
            stream.write_buf(&mut response).await.unwrap();
        }
    } else {
        panic!("Sometgin went wrong");
    }
}
