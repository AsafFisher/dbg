#![no_std]
#![no_main]
#![feature(default_alloc_error_handler)]
//extern crate alloc;
//extern crate compiler_builtins;
use libcore::run;
#[no_mangle]
fn _start() {
    run();
}
