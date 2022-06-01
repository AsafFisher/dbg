use mmap::{
    MapOption::{MapExecutable, MapReadable, MapWritable},
    MemoryMap,
};
use std::mem;

// as the shellcode is not in the `.text` section, we can't execute it as it
const SHELLCODE: &[u8] = include_bytes!("../../text.data");
const word: &str = "Hello world";
fn main() {
    let shellcode_block = MemoryMap::new(SHELLCODE.len(), &[MapReadable, MapWritable, MapExecutable]).unwrap();
    let string_block = MemoryMap::new(word.len(), &[MapReadable, MapWritable, MapExecutable]).unwrap();
    // print word's mem pointer:
    println!("{:p}", string_block.data());
    unsafe {
        // copy the shellcode to the memory map
        std::ptr::copy(SHELLCODE.as_ptr(), shellcode_block.data(), SHELLCODE.len());
        // copy the word to the memory map
        std::ptr::copy(word.as_ptr(), string_block.data(), word.len());
        let exec_shellcode: extern "C" fn() -> ! = mem::transmute(shellcode_block.data());
        exec_shellcode();
    }
}
