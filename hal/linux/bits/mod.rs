use core::ReadWrite;
use std::net::{TcpListener};
use anyhow::{Result};
// use std::io::Write;
// use std::io::Read;
// struct TcpConnection{
//     stream: TcpStream
// }

// impl Connection for TcpConnection{
//     fn write(&self, buf: &[u8]) -> Result<usize> {
//         Ok((&self.stream).write(buf)?)
//     }
//     fn read(&self, buf: &mut [u8]) -> Result<usize> {
//         println!("MEMS");
//         Ok((&self.stream).read(buf)?)
//     }
// }
#[no_mangle]
pub extern "Rust" fn init_connection() -> Result<Box<dyn ReadWrite>>{
    let listener = TcpListener::bind("127.0.0.1:8080");
    // Maybe allow multi connection
    let (stream,addr) = listener.unwrap().accept().unwrap();
    println!("Connected to {:?}", addr);
    Ok(Box::new(stream))
}

#[no_mangle]
pub extern "Rust" fn handle_error(err: anyhow::Error, _connection: &mut dyn ReadWrite) -> Result<()>{
    for _cause in err.chain(){
        // if let Some(err) = cause.downcast_ref::<Error>() {
        //     println!("{:?}", bincode::serialize(err).unwrap().as_slice());
        //     connection.write_all(bincode::serialize(err).unwrap().as_slice());
        //     match err{
        //         Error::BadAddress { address } => {
        //             println!("Address {:X} is fucked up", address);
        //             return Ok(())
        //         }
        //     }
        // }
    }
    Ok(())
}