extern crate alloc;
use core::ffi::c_void;
use core::ops::{BitAnd, Not};

use crate::alloc::string::ToString;
use alloc::string::String;
use rustix::fd::OwnedFd;
use rustix::net::{AddressFamily, Protocol, SocketType};
use rustix::net::{IpAddr, Ipv4Addr, SocketAddr};
const PAGE_SIZE: usize = 0x1000;
pub struct Hal;

pub struct Connection {
    sock: OwnedFd,
}
impl Connection {
    pub fn new(sock_fd: OwnedFd) -> Connection {
        Connection { sock: sock_fd }
    }
}

impl core2::io::Read for Connection {
    fn read(&mut self, _buf: &mut [u8]) -> core2::io::Result<usize> {
        // read from socket libc
        match rustix::io::read(&self.sock, _buf) {
            Ok(n) => Ok(n),
            Err(_) => Err(core2::io::Error::new(
                core2::io::ErrorKind::Other,
                "read error",
            )),
        }
    }
}

impl core2::io::Write for Connection {
    fn write(&mut self, _buf: &[u8]) -> core2::io::Result<usize> {
        // write to socket libc
        match rustix::io::write(&self.sock, _buf) {
            Ok(n) => Ok(n),
            Err(_) => Err(core2::io::Error::new(
                core2::io::ErrorKind::Other,
                "write error",
            )),
        }
    }

    fn flush(&mut self) -> core2::io::Result<()> {
        Ok(())
    }
}

fn align_down(address: usize, size: usize) -> usize {
    address.bitand((size - 1).not())
}

fn mprotect(
    address: *const u8,
    length: usize,
    flags: rustix::mm::MprotectFlags,
) -> Result<(), String> {
    let start_page = align_down(address as usize, PAGE_SIZE);
    match unsafe { rustix::mm::mprotect(start_page as *mut c_void, length, flags) } {
        Ok(_) => Ok(()),
        Err(_) => Err("Could not mprotect.".to_string()),
    }
}

impl Hal {
    pub fn print(s: &str) {
        unsafe {
            rustix::io::write(rustix::io::stdout(), s.as_bytes()).unwrap_or_else(|_| {
                panic!("PTY error");
            })
        };
    }
    pub fn init_connection(port: Option<u16>) -> Result<Connection, ()> {
        // Create a libc socket
        let sock =
            rustix::net::socket(AddressFamily::INET, SocketType::STREAM, Protocol::default())
                .unwrap();
        rustix::net::sockopt::set_socket_reuseaddr(&sock, true).expect("Cant setsockopt");
        // Create SocketAddr
        rustix::net::bind(
            &sock,
            &SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port.unwrap_or(12343)),
        )
        .expect("Could not bind");

        rustix::net::listen(&sock, 1).unwrap();
        let client_sock = rustix::net::accept(&sock).unwrap();
        let listener = Connection::new(client_sock);
        // Maybe allow multi connection
        //println!("Connected to {:?}", addr);
        Ok(listener)
    }

    pub fn enable_write(address: &mut [u8]) -> Result<(), String> {
        // Introduce in the future MemoryRegion, it has as_slice, etc.
        mprotect(
            address.as_mut_ptr(),
            address.len(),
            rustix::mm::MprotectFlags::WRITE.union(rustix::mm::MprotectFlags::READ),
        )
    }
    pub fn disable_write(address: &mut [u8]) -> Result<(), String> {
        // Introduce in the future MemoryRegion, it has as_slice, etc.
        mprotect(
            address.as_mut_ptr(),
            address.len(),
            rustix::mm::MprotectFlags::READ.union(rustix::mm::MprotectFlags::EXEC),
        )
    }
}
