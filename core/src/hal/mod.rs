use alloc::string::String;
use core2;

// TODO: remove no_logic should not be here at all.
#[cfg(any(feature = "linux_um", feature = "no_logic"))]
use hal_linux_um::{Connection as ConnImpl, Hal as HalImpl};
#[cfg(feature = "linux_um_shellcode")]
use hal_linux_um_shellcode::{Connection as ConnImpl, Hal as HalImpl};
pub struct Hal;

pub struct Connection {
    conn: ConnImpl,
}

impl Connection {
    pub fn new(conn: ConnImpl) -> Connection {
        Connection { conn: conn }
    }
}

impl core2::io::Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> core2::io::Result<usize> {
        self.conn.read(buf)
    }
}

impl core2::io::Write for Connection {
    fn write(&mut self, buf: &[u8]) -> core2::io::Result<usize> {
        self.conn.write(buf)
    }

    fn flush(&mut self) -> core2::io::Result<()> {
        self.conn.flush()
    }
}
impl Hal {
    pub fn print(s: &str) {
        HalImpl::print(s)
    }
    pub fn init_connection(port: Option<u16>) -> Result<Connection, ()> {
        Ok(Connection::new(HalImpl::init_connection(port)?))
    }

    pub fn enable_write(address: &mut [u8]) -> Result<(), String> {
        HalImpl::enable_write(address)
    }
    pub fn disable_write(address: &mut [u8]) -> Result<(), String> {
        HalImpl::disable_write(address)
    }
}
