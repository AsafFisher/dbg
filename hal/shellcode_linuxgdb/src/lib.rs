#![no_std]
#![feature(panic_info_message)]
extern crate alloc;
use core::{arch::asm, ptr::NonNull, panic::Location};
use alloc::{boxed::Box, string::ToString};
use anyhow::Result;
use common::ReadWrite;
use core::{marker::PhantomData, panic};
use libcore::Hal;
use syscalls;
use rustix::{self, net::{AddressFamily, SocketType, Protocol}};
#[macro_use]
extern crate sc;

const STDOUT: usize = 1;
static PANIC_MESSAGE: &str = "unknown paniced!\n";
#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    let string = match panic_info.message(){
        Some(s) => s.as_str().unwrap(),
        None => PANIC_MESSAGE,
    };

    loop {
        unsafe {
            syscall!(
                WRITE,
                STDOUT,
                string.as_ptr() as usize,
                string.len()
            );
            asm!("int 3");
        }
    }
}

struct LinuxHal {
    sock: usize,
}
impl LinuxHal {
    fn new(sock_fd: usize) -> LinuxHal {
        LinuxHal { sock: sock_fd}
    }
}


impl core2::io::Read for LinuxHal {
    fn read(&mut self, _buf: &mut [u8]) -> core2::io::Result<usize> {
        // read from socket libc
        let res = unsafe {
            // read from socket
            syscall!(RECVFROM, self.sock, _buf.as_mut_ptr(), _buf.len(), 0, 0, 0)
        };
        if res <= 0 {
            Err(core2::io::Error::new(core2::io::ErrorKind::Other, "read error"))
        } else {
            Ok(res as usize)
        }
    }
}

impl core2::io::Write for LinuxHal {
    fn write(&mut self, _buf: &[u8]) -> core2::io::Result<usize> {
        // write to socket libc
        let res = unsafe {
            // write to socket
            syscall!(SENDTO, self.sock, _buf.as_ptr(), _buf.len(), 0, 0, 0)
        };
        if res < 0 {
            panic!("write failed");
        } else {
            Ok(res as usize)
        }
    }

    fn flush(&mut self) -> core2::io::Result<()> {
        Ok(())
    }
}

unsafe fn syscall3(syscall: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        in("rax") syscall,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        out("rcx") _,
        out("r11") _,
        lateout("rax") ret,
        options(nostack),
    );
    ret
}

impl Hal<LinuxHal> for LinuxHal {
    fn print(&self, s: &str) {
        let res = unsafe {
            syscall!(
                WRITE,
                STDOUT,
                s.as_ptr() as usize,
                s.len()
            )
        };
        if res < 0 {
            panic!("write failed");
        }
    }
    fn init_connection(&self) -> Result<Box<LinuxHal>> {
        // Create a libc socket
        let sock = unsafe {
            // match syscalls::syscall3(syscalls::Sysno::socket, libc::AF_INET as usize, libc::SOCK_STREAM as usize, 0){
            //     Ok(fd) => fd,
            //     Err(err) => panic!("socket error: {}", err),
            // }
            syscall!(SOCKET, libc::AF_INET as usize, libc::SOCK_STREAM as usize, 0)
        };
        // check if socket is valid
        if sock < 0 {
            panic!("socket error");
        }
        unsafe{
            //libc sockaddr localhost
            let addr = libc::sockaddr_in {
                sin_family: libc::AF_INET as u16,
                sin_port: 12343_u16.to_be(),
                sin_addr: libc::in_addr { s_addr: 0 },
                sin_zero: [0; 8],
            };
            // bind syscall to socket
            let res = syscall!(BIND, sock, &addr as *const _ as usize, core::mem::size_of::<libc::sockaddr_in>() as usize);
            // Check if bind was successful
            if res < 0 {
                panic!("bind failed");
            }
        };
        // check if bind was successful
        if sock < 0 {
            panic!("bind failed");
        }
        unsafe{
            // listen syscall to socket
            let res = syscall!(LISTEN, sock, 1);
            // Check if listen was successful
            if res < 0 {
                panic!("listen failed");
            }
        }
        let client_sock = unsafe{
            // set sockaddr to null pointer
            let addr: libc::sockaddr_in = core::mem::zeroed();
            // accept socket
            let client_sock = syscall!(ACCEPT, sock, 0, 0);//&addr as *const _ as usize, core::mem::size_of::<libc::sockaddr_in>() as usize);
            // check accept
            if client_sock < 0 {
                panic!("accept failed");
            }
            client_sock
        };
        let listener = LinuxHal::new(client_sock);
        // Maybe allow multi connection
        //println!("Connected to {:?}", addr);
        Ok(Box::new(listener))
    }

    fn handle_error(&self, _err: anyhow::Error, _connection: &mut LinuxHal) -> Result<()> {
        Ok(())
    }
}

#[inline]
pub fn hal_run() {
    let hal = LinuxHal::new(0);
    libcore::run(hal);
}
