#![feature(slice_pattern)]
#![no_std]
#![feature(asm)]
extern crate alloc;
extern crate base64;

//mod io_impl;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::{format, vec};
use alloc::{string::ToString, vec::Vec};
use anyhow::Result;
pub use base64::{decode, encode};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use common::{ReadWrite};
use core2::io::{Write, Read};
use core::{ffi::c_void, slice::SlicePattern};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::Deserialize;
use static_alloc::Bump;
use core::arch::asm;
use minicbor;

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
enum Response{
    #[n(0)] BytesRead{
        #[n(0)] buff: minicbor::bytes::ByteVec,
    },
    #[n(1)] BytesWritten{
        #[n(0)] written: u64,
    },
    #[n(2)] FunctionExecuted{
        #[n(0)] ret: u64,
    },
}
#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
enum ResponseStatus {
    #[n(0)] Success { #[n(0)] response: Response},
    #[n(1)] Error,
    
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]

struct WriteCmd{
    #[n(0)] address: u64,
    #[n(1)] buff: minicbor::bytes::ByteVec,
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
struct ReadCmd{
    #[n(0)] address: u64,
    #[n(1)] size: u64,
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
struct CallCmd{
    #[n(0)] address: u64,
    #[n(1)] argunments: Vec<u64>,
}

#[global_allocator]
static A: Bump<[u8; 1 << 16]> = Bump::uninit();


// TODO: move cmd structs to enum 
#[derive(FromPrimitive, Deserialize)]
pub enum CMD {
    READ = 0,
    WRITE = 1,
    CALL = 2,
}

pub trait Hal<RW: ReadWrite> {
    fn print(&self, s: &str);
    fn init_connection(&self) -> Result<Box<RW>>;
    fn handle_error(&self, _err: &anyhow::Error, _connection: &mut RW) -> Result<()>;
}

struct Engine<RW: ReadWrite, T: Hal<RW>> {
    hal: T,
    phantom: core::marker::PhantomData<RW>,
}
impl<RW: ReadWrite, T: Hal<RW>> Engine<RW, T> {
    pub fn new(hal: T) -> Self {
        Self {
            hal,
            phantom: core::marker::PhantomData,
        }
    }
    pub fn run(&self) {
        loop {
            self.hal.print("waiting for connection\n");
            let mut connection = self.hal.init_connection().unwrap();
            match self.handle_client(&mut *connection) {
                Ok(()) => return,
                Err(()) => continue,
                // Err(err) => match self.hal.handle_error(&err, &mut *connection) {
                //     Ok(()) => continue,
                //     Err(_) => todo!(),
                // },
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
    fn handle_write(&self, message: &[u8]) -> Result<Response> {
        let write_cmd: WriteCmd  = minicbor::decode(message).unwrap();
        unsafe {
            let slice = core::slice::from_raw_parts_mut(write_cmd.address as *mut u8, write_cmd.buff.len());
            // copy byte by byte from buff to slice
            slice.copy_from_slice(write_cmd.buff.as_slice());
        }

        Ok(Response::BytesWritten{ written: write_cmd.buff.len() as u64 })
    }

    fn handle_read(&self, message: &[u8]) -> Result<Response> {
        let read_cmd: ReadCmd = minicbor::decode(message).unwrap();
        unsafe {
            let mut read_buff = alloc::vec::Vec::<u8>::new();
            // write_all might not be able to check that the buffer has the correct address.
            match read_buff.write_all(core::slice::from_raw_parts(read_cmd.address as *mut u8, read_cmd.size as usize)) {
                Ok(_) => {
                    Ok(Response::BytesRead{buff: minicbor::bytes::ByteVec::from(read_buff)})
                },
                // TODO: make sure this is ok
                Err(_) => {
                    Err(anyhow::anyhow!("Error reading from address"))
                }
            }
        }
    }

    fn make_call(
        &self,
        ptr: *const c_void,
        mut argunments: Vec<u64>,
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
                1 => abi_call!(u64),
                2 => abi_call!(u64, u64),
                3 => abi_call!(u64, u64, u64),
                4 => abi_call!(u64, u64, u64, u64),
                5 => abi_call!(u64, u64, u64, u64, u64),
                6 => abi_call!(u64, u64, u64, u64, u64, u64),
                7 => abi_call!(u64, u64, u64, u64, u64, u64, u64),
                8 => abi_call!(u64, u64, u64, u64, u64, u64, u64, u64),
                9 => abi_call!(u64, u64, u64, u64, u64, u64, u64, u64, u64),
                10 => {
                    abi_call!(u64, u64, u64, u64, u64, u64, u64, u64, u64, u64)
                }
                _ => panic!("Wrong"),
            }
        };
        Ok(ret_val)
    }
    
    fn handle_call(&self, message: &[u8]) -> Result<Response> {
        let call_cmd: CallCmd = minicbor::decode(message).unwrap();
        let mut response = alloc::vec::Vec::<u8>::new();

        let ret_val = self.make_call(call_cmd.address as *const c_void, call_cmd.argunments);
        if let Ok(ret_val) = ret_val {
            Ok(Response::FunctionExecuted { ret: ret_val })
        } else {
            Err(anyhow::anyhow!("Error executing function"))
        }
    }

    pub fn handle_client(&self, connection: &mut RW) -> core::result::Result<(),()> {
        loop {
            let code = match connection.read_u32::<LittleEndian>() {
                Ok(code) => code,
                Err(err) => {
                    self.hal.print("Restarting service\n");
                    return Err(());
                }
            };

            let message_slc = self.read_msg_buffer(connection);
            
            // TODO: remove unwrap
            let res = match FromPrimitive::from_u32(code) {
                Some(CMD::READ) => self.handle_read(message_slc.as_slice()),
                Some(CMD::WRITE) => {
                    self.handle_write(message_slc.as_slice())
                },
                Some(CMD::CALL) => self.handle_call(message_slc.as_slice()),
                None => todo!(),
            };

            //TODO: FINISH
            let res = match res {
                Ok(response) => {
                    ResponseStatus::Success { response: response }
                },
                Err(err) => {
                    // Error type import
                    ResponseStatus::Error
                }, //handle_error(err, &mut *connection)?,
            };
            let mut res_buf = alloc::vec::Vec::<u8>::new();
            minicbor::encode(res, &mut res_buf).unwrap();
            connection.write_u64::<LittleEndian>(res_buf.len() as u64).unwrap();
            connection.write(res_buf.as_slice()).unwrap();
        }
    }
}

pub fn run<RW: ReadWrite, T: Hal<RW>>(hal: T) {
    Engine::new(hal).run();
}
