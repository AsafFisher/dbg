use anyhow::Result;
use core::ReadWrite;
use std::net::TcpListener;
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
pub fn init_connection() -> Result<Box<dyn ReadWrite>> {
    let listener = TcpListener::bind("127.0.0.1:8080");
    // Maybe allow multi connection
    let (stream, addr) = listener.unwrap().accept().unwrap();
    println!("Connected to {:?}", addr);
    Ok(Box::new(stream))
}

#[no_mangle]
pub fn handle_error(err: anyhow::Error, _connection: &mut dyn ReadWrite) -> Result<()> {
    for cause in err.chain() {
        if let Some(io_error) = cause.downcast_ref::<std::io::Error>() {
            if io_error.kind() == std::io::ErrorKind::UnexpectedEof {
                println!("Restarting remote debugger");
                continue;
            } else {
                return Err(anyhow::anyhow!("IO ERROR: {:?}", err))
            }
        } else {
            return Err(anyhow::anyhow!("FATAL: {:?}", err))
        }
    }
    Ok(())
}
