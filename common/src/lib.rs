#![no_std]
use core2::io::{Read, Write};
pub trait ReadWrite: Read + Write {}
impl<T: Read + Write + ?Sized> ReadWrite for T {}
