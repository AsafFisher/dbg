#![no_std]
use core2::io;
pub trait ReadWrite: io::Read + io::Write {}
impl<T: io::Read + io::Write + ?Sized> ReadWrite for T {}
pub trait Read: io::Read {}
impl<R: io::Read + ?Sized> Read for R {}
pub trait Write: io::Write {}
impl<W: io::Write + ?Sized> Write for W {}
