#[allow(unused_imports)]
use std::env;
use std::fmt::{self, Display, Formatter};
#[allow(unused_imports)]
use std::fs;
use std::io::BufRead;

use bytes::Bytes;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

#[derive(Debug, PartialEq, PartialOrd)]
#[allow(dead_code)]
enum RespValue {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(String),
    Array(Vec<RespValue>),
    Null,
}

#[allow(dead_code)]
impl RespValue {
    fn try_from_reader(mut r: impl BufRead) -> Result<Self, &'static str> {
        let mut first_char: [u8; 1] = [0];

        r.read_exact(&mut first_char).unwrap();

        match first_char[0] as char {
            '+' => {
                let mut buffer = String::new();
                r.read_line(&mut buffer).unwrap();
                Ok(RespValue::SimpleString(buffer))
            }
            '-' => {
                let mut buffer = String::new();
                r.read_line(&mut buffer).unwrap();
                Ok(RespValue::Error(buffer))
            }
            ':' => {
                let mut buffer = String::new();
                r.read_line(&mut buffer).unwrap();
                Ok(RespValue::Integer(buffer.parse().unwrap()))
            }
            '$' => {
                let mut buffer = String::new();
                r.read_line(&mut buffer).unwrap();

                let size = buffer.parse().unwrap();
                let mut buffer = Vec::with_capacity(size);

                r.read_exact(&mut buffer).unwrap();

                Ok(RespValue::BulkString(String::from_utf8(buffer).unwrap()))
            }
            '*' => {
                let mut buffer = String::new();
                r.read_line(&mut buffer).unwrap();

                let size = buffer.parse().unwrap();
                let mut values = Vec::with_capacity(size);

                for _ in 0..size {
                    values.push(RespValue::try_from_reader(&mut r)?);
                }

                Ok(RespValue::Array(values))
            }
            _ => Err("Something went wrong"),
        }
    }
    fn as_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(self.to_string().as_bytes())
    }
}

impl Display for RespValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimpleString(s) => write!(f, "+{}\r\n", s),
            Self::Error(e) => write!(f, "-{}\r\n", e),
            Self::Integer(i) => write!(f, ":{}\r\n", i),
            Self::BulkString(s) => write!(f, "${}\r\n{}\r\n", s.len(), s),
            Self::Array(v) => {
                write!(f, "*{}\r\n", v.len())?;
                for resp in v {
                    write!(f, "{}", resp)?;
                }
                write!(f, "\r\n")
            }
            Self::Null => write!(f, "$-1\r\n"),
        }
    }
}

#[tokio::main]
async fn main() {
    let mut listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    match listener.accept().await {
        Ok((mut socket, _addr)) => {
            socket
                .write_buf(&mut RespValue::SimpleString("PONG".to_string()).as_bytes())
                .await
                .unwrap();
        }
        Err(e) => panic!("Something went wrong\n{}", e),
    }
}

// async fn handle_message(msg: impl AsyncReadExt + std::pin::) {
//     let mut buffer = BytesMut::with_capacity(4);
//     let data = msg.read_exact(&mut buffer);
// }
