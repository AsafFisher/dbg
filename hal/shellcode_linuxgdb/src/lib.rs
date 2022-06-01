#![no_std]
#![feature(panic_info_message)]
#![feature(core_intrinsics)]
extern crate alloc;
use alloc::boxed::Box;
use core::arch::asm;
use libcore::Hal;
use rustix::io::OwnedFd;
use rustix::net::{AddressFamily, Protocol, SocketType};
use rustix::net::{IpAddr, Ipv4Addr, SocketAddr};

//const STDOUT: usize = 1;
static PANIC_MESSAGE: &str = "unknown paniced!\n";
#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    let _string = match panic_info.message() {
        Some(s) => s.as_str().unwrap(),
        None => PANIC_MESSAGE,
    };

    loop {
        unsafe {
            //rustix::fs::
            //syscall!(WRITE, STDOUT, string.as_ptr() as usize, string.len());
            core::intrinsics::breakpoint();
        }
    }
}

struct LinuxHal;

struct LinuxConnection {
    sock: OwnedFd,
}
impl LinuxConnection {
    fn new(sock_fd: OwnedFd) -> LinuxConnection {
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

impl Hal<LinuxConnection> for LinuxHal {
    fn print(_s: &str) {
        // let res = unsafe { syscall!(WRITE, STDOUT, s.as_ptr() as usize, s.len()) };
        // if (res as isize) < 0 {
        //     panic!("write failed");
        // }
    }
    fn init_connection() -> Result<Box<LinuxConnection>, ()> {
        // Create a libc socket
        let sock =
            rustix::net::socket(AddressFamily::INET, SocketType::STREAM, Protocol::default())
                .unwrap();
        // Create SocketAddr
        rustix::net::bind(
            &sock,
            &SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 12343),
        )
        .unwrap();
        rustix::net::listen(&sock, 1).unwrap();
        let client_sock = rustix::net::accept(&sock).unwrap();
        let listener = LinuxConnection::new(client_sock);
        // Maybe allow multi connection
        //println!("Connected to {:?}", addr);
        Ok(Box::new(listener))
    }

    fn handle_error(_err: &str, _connection: &mut LinuxConnection) -> Result<(), ()> {
        Ok(())
    }
}

#[inline]
pub fn hal_run() {
    libcore::run::<LinuxConnection, LinuxHal>();
}
