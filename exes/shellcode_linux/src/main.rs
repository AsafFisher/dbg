#![no_std]
#![no_main]
//extern crate alloc;
//extern crate compiler_builtins;
use libcore::run;
#[no_mangle]
fn _start() {
    run();
}
