#![feature(slice_pattern)]
#![feature(type_alias_impl_trait)]
#![feature(core_intrinsics)]
#![no_std]
extern crate alloc;
mod arch;
pub mod comm;
pub mod engine;
mod hal;
mod hooks;
