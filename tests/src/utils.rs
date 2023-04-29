use mmap::{
    MapOption::{MapExecutable, MapReadable, MapWritable},
    MemoryMap,
};
use std::mem::transmute;

pub fn generate_read_write_exec_page(data: &[u8]) -> MemoryMap {
    let mapped = MemoryMap::new(data.len(), &[MapReadable, MapWritable, MapExecutable]).unwrap();
    unsafe { std::ptr::copy(data.as_ptr(), mapped.data(), data.len()) };
    mapped
}

pub fn run_shellcode(shellcode_block: *mut u8) {
    unsafe {
        let exec_shellcode: extern "C" fn(base_addr: usize) = transmute(shellcode_block);
        exec_shellcode(shellcode_block as usize);
    }
}
