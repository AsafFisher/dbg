
// extern {
//     fn open() -> u32;
// }
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use byteorder::{BigEndian, ReadBytesExt};
enum CMD{
    READ = 0;
}
fn main() -> std::io::Result<()>{

    let listener = TcpListener::bind("127.0.0.1:8080")?;

    // accept connections and process them serially
    for stream in listener.incoming() {
        handle_client(stream?);
    }
    Ok(())
}
fn handle_client(mut stream: TcpStream){
    let cmd = [0; 4];    
    let a: u32 = stream.read_u32::<BigEndian>().unwrap();
    println!("{:?}", a);
}
//link-arg=