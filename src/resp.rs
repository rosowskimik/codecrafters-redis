use std::fmt::{self, Display, Formatter};

use bytes::{Buf, BufMut, Bytes, BytesMut};

#[derive(Debug, Clone, PartialEq)]
pub enum RespValue {
    Simple(String),
    Error(String),
    Integer(u64),
    Bulk(String),
    Null,
    Array(Vec<RespValue>),
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum RespError {
    Incomplete,
    WrongKind {
        expected: &'static str,
        got: &'static str,
    },
    Other,
}

#[allow(dead_code)]
impl RespValue {
    pub fn try_from_bytes(bytes: &mut BytesMut) -> Result<Self, RespError> {
        if !bytes.has_remaining() {
            return Err(RespError::Incomplete);
        }

        match bytes[0] {
            b'+' => {
                let line = read_line(bytes)?;
                Ok(RespValue::Simple(line))
            }
            b'-' => {
                let line = read_line(bytes)?;
                Ok(RespValue::Error(line))
            }
            b':' => {
                let line = read_line(bytes)?;
                Ok(RespValue::Integer(line.parse().unwrap()))
            }
            b'$' => {
                if bytes.starts_with(b"$-1\r\n") {
                    bytes.advance(5);
                    return Ok(RespValue::Null);
                }

                let line = peek_line(bytes)?;
                let size_len = line.len();
                let size = line.parse().unwrap();

                if bytes.remaining() < line.len() + size + 5 {
                    return Err(RespError::Incomplete);
                }
                bytes.advance(size_len + 3);

                let bs = String::from_utf8((&bytes[..size]).to_vec()).unwrap();
                bytes.advance(size + 2);

                Ok(RespValue::Bulk(bs))
            }
            b'*' => {
                if bytes.starts_with(b"*-1\r\n") {
                    bytes.advance(5);
                    return Ok(RespValue::Null);
                }
                // todo!()
                let line = peek_line(bytes)?;
                let size = line.parse().unwrap();
                let mut inner = Vec::with_capacity(size);
                let mut inner_size = 0;

                {
                    let mut value = bytes.clone();
                    value.advance(line.as_bytes().len() + 3);
                    for _ in 0..size {
                        let len = value.len();
                        inner.push(RespValue::try_from_bytes(&mut value)?);
                        inner_size += len - value.len();
                    }
                }

                bytes.advance(inner_size + 1);
                Ok(RespValue::Array(inner))
            }
            _ => Err(RespError::Other),
        }
    }

    pub fn new_simple(value: impl ToString) -> Self {
        Self::Simple(value.to_string())
    }

    pub fn new_error(value: impl ToString) -> Self {
        Self::Error(value.to_string())
    }

    pub fn new_integer(value: impl Into<u64>) -> Self {
        Self::Integer(value.into())
    }

    pub fn new_bulk(value: impl ToString) -> Self {
        Self::Bulk(value.to_string())
    }

    pub fn new_array() -> Self {
        Self::Array(Vec::new())
    }

    pub fn array_with<T>(value: Vec<T>) -> Self
    where
        RespValue: From<T>,
    {
        Self::Array(value.into_iter().map(RespValue::from).collect())
    }

    pub const fn kind(&self) -> &'static str {
        match *self {
            Self::Simple(_) => "simple string",
            Self::Error(_) => "error",
            Self::Integer(_) => "integer",
            Self::Bulk(_) => "bulk string",
            Self::Null => "null",
            Self::Array(_) => "array",
        }
    }

    pub fn raw_bytes(&self) -> Bytes {
        let mut buf = match self {
            Self::Simple(s) => {
                let val = s.as_bytes();
                let mut buf = BytesMut::with_capacity(val.len() + 3);
                buf.put_u8(b'+');
                buf.put(val);
                buf
            }
            Self::Error(e) => {
                let val = e.as_bytes();
                let mut buf = BytesMut::with_capacity(val.len() + 3);
                buf.put_u8(b'-');
                buf.put(val);
                buf
            }
            Self::Integer(i) => {
                let val = i.to_string();
                let val = val.as_bytes();
                let mut buf = BytesMut::with_capacity(val.len() + 3);
                buf.put_u8(b':');
                buf.put(val);
                buf
            }
            Self::Bulk(s) => {
                let val = s.as_bytes();
                let len = val.len();
                let len_bytes = len.to_string();
                let len_bytes = len_bytes.as_bytes();
                let mut buf = BytesMut::with_capacity(len + len_bytes.len() + 5);

                buf.put_u8(b'$');
                buf.put(len_bytes);
                buf.put_slice(b"\r\n");
                buf.put(val);
                buf
            }
            Self::Array(v) => {
                let len = v.len();
                let len_bytes = len.to_string();
                let len_bytes = len_bytes.as_bytes();
                let val = v.iter().map(|resp| resp.raw_bytes()).fold(
                    BytesMut::new(),
                    |mut acc, bytes| {
                        acc.put(bytes);
                        acc
                    },
                );
                let mut buf = BytesMut::with_capacity(len_bytes.len() + val.len() + 5);
                buf.put_u8(b'*');
                buf.put(len_bytes);
                buf.put_slice(b"\r\n");
                buf.put_slice(&val);
                buf
            }
            Self::Null => {
                let mut buf = BytesMut::with_capacity(5);
                buf.put_slice(b"$-1");
                buf
            }
        };
        buf.put_slice(b"\r\n");

        buf.freeze()
    }

    // pub fn try_into_command(self) -> Result<String, Error> {

    // }

    pub fn array_push(&mut self, val: RespValue) -> Result<(), RespError> {
        assert!(matches!(*self, Self::Array(_)));
        if let Self::Array(v) = self {
            v.push(val);
            Ok(())
        } else {
            Err(RespError::WrongKind {
                expected: Self::new_array().kind(),
                got: self.kind(),
            })
        }
    }
}

fn peek_line(buf: &BytesMut) -> Result<&str, RespError> {
    let start = 1;
    let end = buf.len() - 1;
    for i in start..end {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' {
            return Ok(std::str::from_utf8(&buf[1..i]).unwrap());
        }
    }
    Err(RespError::Incomplete)
}

fn read_line(buf: &mut BytesMut) -> Result<String, RespError> {
    let start = 1;
    let end = buf.len() - 1;
    for i in start..end {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' {
            let val = String::from_utf8((&buf[1..i]).to_vec()).unwrap();
            buf.advance(i + 2);
            return Ok(val);
        }
    }
    Err(RespError::Incomplete)
}

impl std::error::Error for RespError {}

impl Display for RespError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete => "Stream ended early".fmt(f),
            Self::WrongKind { expected, got } => {
                write!(f, "Wrong Kind (expected '{}' got '{}')", expected, got)
            }
            Self::Other => "Something went wrong".fmt(f),
        }
    }
}
