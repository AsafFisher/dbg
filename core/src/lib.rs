#![feature(slice_pattern)]
#![no_std]
#![feature(asm)]
extern crate alloc;
extern crate base64;

use alloc::boxed::Box;
use alloc::format;
use alloc::{string::ToString, vec::Vec};
use anyhow::Result;
pub use base64::{decode, encode};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use common::ReadWrite;
use core::{ffi::c_void, slice::SlicePattern};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::Deserialize;
use static_alloc::Bump;

#[global_allocator]
static A: Bump<[u8; 1 << 16]> = Bump::uninit();

#[derive(FromPrimitive, Deserialize)]
pub enum CMD {
    READ = 0,
    WRITE = 1,
    CALL = 2,
}

#[derive(Deserialize, Debug)]
pub struct CallCmd {
    address: usize,
    parameters: [usize; 10],
}


pub trait Hal<RW: ReadWrite> {
    fn print(&self, s: &str);
    fn init_connection(&self) -> Result<Box<RW>>;
    fn handle_error(&self, _err: anyhow::Error, _connection: &mut RW) -> Result<()>;
}
struct Engine <RW: ReadWrite, T: Hal<RW>> {
    hal: T,
    phantom: core::marker::PhantomData<RW>,
}
impl <RW: ReadWrite, T: Hal<RW>> Engine <RW, T> {
    pub fn new(hal: T) -> Self {
        Self {
            hal,
            phantom: core::marker::PhantomData,
        }
    }
    pub fn run(&self) {
        loop {
            let mut connection = self.hal.init_connection().unwrap();
            match self.handle_client(&mut *connection) {
                Ok(()) => return,
                Err(err) => match self.hal.handle_error(err, &mut *connection) {
                    Ok(()) => continue,
                    Err(_) => todo!(),
                },
            };
        }
    }
    fn read_msg_buffer(&self, connection: &mut RW) -> Vec<u8> {
        let msg_size = connection.read_u64::<LittleEndian>().unwrap();
        let mut buff = Vec::with_capacity(msg_size as usize);
        buff.resize(msg_size as usize, 0);
        // Unwrap - get rid
        connection.read_exact(buff.as_mut_slice()).unwrap();
    
        buff
    }
    fn handle_write(&self, connection: &mut RW) -> Result<()> {
        let address = self.read_msg_buffer(connection)
            .as_slice()
            .read_u64::<LittleEndian>()
            .unwrap();
        let buff = self.read_msg_buffer(connection);
    
        unsafe {
            let slice = core::slice::from_raw_parts_mut(address as *mut u8, buff.len());
    
            // copy byte by byte from buff to slice
            slice.copy_from_slice(buff.as_slice());
        }
    
        Ok(())
    }
    
    fn handle_read(&self, connection: &mut RW) -> Result<()> {
        let mut buff = [0; core::mem::size_of::<usize>()];
        connection.read_exact(&mut buff).unwrap();
    
        // ptr
        let mut buff1 = [0; core::mem::size_of::<usize>()];
        connection.read_exact(&mut buff1).unwrap();
    
        let raw_pointer = usize::from_ne_bytes(buff1) as *mut u8;
        let len = usize::from_ne_bytes(buff);
        unsafe {
            // write_all might not be able to check that the buffer has the correct address.
            match connection.write_all(core::slice::from_raw_parts(raw_pointer, len)) {
                Ok(_) => Ok(()),
                // TODO: make sure this is ok
                Err(_) => Err(anyhow::anyhow!(
                    "Somthing went wring in writing to connection"
                )),
            }
        }
    }
    
    fn make_call(
        &self,
        connection: &mut dyn ReadWrite,
        ptr: *const c_void,
        mut argunments: Vec<usize>
    ) -> Result<u64> {
        // ABI call macro
        macro_rules! abi_call {
            ($($args:ty),*) => {
                core::mem::transmute::<_, extern "C" fn($($args),*) -> u64>(ptr)(
                    $(abi_call!(@ar $args)),*
                )
            };
            (@ar $x:ty) => {
                argunments.remove(0)
            }
        }
    
        // CODE:
        if argunments.len() >= 11 {
            return Err(anyhow::anyhow!("Too many parameters"));
        }
        let ret_val = unsafe {
            match argunments.len() {
                0 => abi_call!(),
                1 => abi_call!(usize),
                2 => abi_call!(usize, usize),
                3 => abi_call!(usize, usize, usize),
                4 => abi_call!(usize, usize, usize, usize),
                5 => abi_call!(usize, usize, usize, usize, usize),
                6 => abi_call!(usize, usize, usize, usize, usize, usize),
                7 => abi_call!(usize, usize, usize, usize, usize, usize, usize),
                8 => abi_call!(usize, usize, usize, usize, usize, usize, usize, usize),
                9 => abi_call!(usize, usize, usize, usize, usize, usize, usize, usize, usize),
                10 => abi_call!(usize, usize, usize, usize, usize, usize, usize, usize, usize, usize),
                _ => panic!("Wrong"),
            }
        };
        Ok(ret_val)
    }
    
    fn handle_call(&self, connection: &mut RW) -> Result<()> {
        let address = connection.read_u64::<LittleEndian>().unwrap();
        let param_count = connection.read_u64::<LittleEndian>().unwrap();
        let mut params = Vec::<usize>::with_capacity(param_count as usize);
        
        params.resize(param_count as usize, 0);
        for i in 0..param_count {
            let mut buf = [0; core::mem::size_of::<usize>()];
            connection.read_exact(&mut buf).unwrap();
            // read usize from buff
            params[i as usize] = usize::from_ne_bytes(buf);
        }
        let ret_val = self.make_call(
            connection,
            address as *const c_void,
            params
        );
        if let Ok(ret_val) = ret_val {
            connection.write_u64::<LittleEndian>(ret_val).unwrap();
        }
        Ok(())
    }
    
    pub fn handle_client(&self, connection: &mut RW) -> Result<()> {
        loop {
            let code = match connection.read_u32::<LittleEndian>() {
                Ok(code) => code,
                Err(err) => {
                    todo!("Handler");
                    continue;
                }
            };
    
            // TODO: remove unwrap
            let res = match FromPrimitive::from_u32(code) {
                Some(CMD::READ) => self.handle_read(connection),
                Some(CMD::WRITE) => self.handle_write(connection),
                Some(CMD::CALL) => self.handle_call(connection),
                None => todo!(),
            };
            match res {
                Ok(()) => continue,
                Err(_) => todo!(), //handle_error(err, &mut *connection)?,
            }
        }
    }
}
pub fn run<RW: ReadWrite, T: Hal<RW>>(hal: T) {
    Engine::new(hal).run();
}
