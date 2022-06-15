#![no_std]
#![no_main]
#![feature(default_alloc_error_handler)]
//extern crate alloc;
//extern crate compiler_builtins;
use hal_shellcode_linuxgdb::init_connection;

#[no_mangle]
fn _start() {
    let a = init_connection();
}
