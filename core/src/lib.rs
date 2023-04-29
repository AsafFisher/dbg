#![feature(slice_pattern)]
#![feature(type_alias_impl_trait)]
#![feature(core_intrinsics)]
#![feature(maybe_uninit_as_bytes)]
#![no_std]
extern crate alloc;
#[cfg(not(feature = "no_logic"))]
mod arch;
pub mod comm;

#[cfg(not(feature = "no_logic"))]
pub mod engine;

mod hal;

#[cfg(all(not(feature = "no_logic"), feature = "hooks"))]
mod hooks;

#[cfg(not(feature = "no_logic"))]
#[no_mangle]
pub extern "C" fn _umm_critical_entry(id: *const u32) {}

#[cfg(not(feature = "no_logic"))]
#[no_mangle]
pub extern "C" fn _umm_critical_exit(id: *const u32) {}

// Might want to compile umm-malloc with -fno-stack-protector-all
#[cfg(not(feature = "no_logic"))]
#[no_mangle]
pub extern "C" fn __stack_chk_fail() {
    panic!("Stack Smash Detected");
}
