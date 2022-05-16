#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::net::TcpListener;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379")?;
    match listener.accept() {
        Ok((_socket, addr)) => println!("accepted new client: {:?}", addr),
        Err(e) => return Err(Box::new(e)),
    }

    Ok(())
}
