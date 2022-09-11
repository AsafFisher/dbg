#![no_std]
#![feature(panic_info_message)]
#![feature(core_intrinsics)]
extern crate alloc;
use alloc::boxed::Box;
use rustix::io::OwnedFd;
use rustix::net::{AddressFamily, Protocol, SocketType};
use rustix::net::{IpAddr, Ipv4Addr, SocketAddr};

//const STDOUT: usize = 1;
static PANIC_MESSAGE: &str = "unknown paniced!\n";
#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    let string = match panic_info.message() {
        Some(s) => s.as_str().unwrap(),
        None => PANIC_MESSAGE,
    };

    loop {
        unsafe {
            rustix::io::write(rustix::io::stdout(), string.as_bytes()).unwrap_unchecked();
            core::intrinsics::breakpoint();
        }
    }
}

pub struct Hal;

pub struct LinuxConnection {
    sock: OwnedFd,
}
impl LinuxConnection {
    pub fn new(sock_fd: OwnedFd) -> LinuxConnection {
        LinuxConnection { sock: sock_fd }
    }
}

impl core2::io::Read for LinuxConnection {
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

impl core2::io::Write for LinuxConnection {
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

impl Hal {
    pub fn print(s: &str) {
        unsafe {
            rustix::io::write(rustix::io::stdout(), s.as_bytes()).unwrap_or_else(|_| {
                panic!("PTY error");
            })
        };
    }
    pub fn init_connection() -> Result<Box<LinuxConnection>, ()> {
        // Create a libc socket
        let sock =
            rustix::net::socket(AddressFamily::INET, SocketType::STREAM, Protocol::default())
                .unwrap();

        rustix::net::sockopt::set_socket_reuseaddr(&sock, true).expect("Cant setsockopt");
        // Create SocketAddr
        rustix::net::bind(
            &sock,
            &SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 12343),
        )
        .expect("Could not bind");

        rustix::net::listen(&sock, 1).unwrap();
        let client_sock = rustix::net::accept(&sock).unwrap();
        let listener = LinuxConnection::new(client_sock);
        // Maybe allow multi connection
        //println!("Connected to {:?}", addr);
        Ok(Box::new(listener))
    }
}
