#![feature(slice_pattern)]
#![no_std]

extern crate alloc;
extern crate base64;

use alloc::vec::Vec;
use alloc::boxed::Box;
use anyhow::{Result};
pub use base64::{decode, encode};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use common::ReadWrite;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::Deserialize;
use static_alloc::Bump;
use core::ffi::c_void;

#[global_allocator]
static A: Bump<[u8; 1 << 16]> = Bump::uninit();

//#[cfg(feature = "std")]
//use hal_stdgdb::{init_connection, handle_error};

// #[cfg(not(feature = "std"))]
// use hal_shellcode_linuxgdb::{handle_error, init_connection};

#[derive(FromPrimitive, Deserialize)]
pub enum CMD {
    READ,
    WRITE,
    CALL,
}

#[derive(Deserialize, Debug)]
pub struct ReadCmd {
    size: usize,
    address: usize,
}

#[derive(Deserialize, Debug)]
pub struct CallCmd {
    address: usize,
    parameters: [usize; 10],
}

#[derive(Deserialize, Debug)]
pub struct WriteCmd<'a> {
    address: usize,
    buffer: &'a str,
}

// extern "Rust" {
//     fn init_connection::<T: ReadWrite>() -> Result<T>;
//     fn handle_error(err: anyhow::Error, connection: &mut dyn ReadWrite) -> Result<()>;
// }

fn read_msg_buffer(connection: &mut dyn ReadWrite) -> Vec<u8> {
    let mut buff = [0; core::mem::size_of::<usize>()];
    connection.read_exact(&mut buff).unwrap();
    let msg_size = usize::from_ne_bytes(buff);

    // Make sure u64 == usize
    let mut buff = Vec::with_capacity(msg_size as usize);
    buff.resize(msg_size, 0);

    // Unwrap - get rid
    connection.read_exact(buff.as_mut_slice()).unwrap();

    buff
}
fn handle_write(connection: &mut dyn ReadWrite) -> Result<()> {
    let buff = read_msg_buffer(connection);

    let (cmd, _) = serde_json_core::de::from_slice::<WriteCmd>(buff.as_slice()).unwrap();
    unsafe {
        let slice = core::slice::from_raw_parts_mut(cmd.address as *mut u8, cmd.buffer.len());
        slice.copy_from_slice(decode(&cmd.buffer).unwrap().as_slice())
    }

    Ok(())
}

fn handle_read(connection: &mut dyn ReadWrite) -> Result<()> {
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
    connection: &mut dyn ReadWrite,
    ptr: *const c_void,
    mut argunments: Vec<usize>,
) -> Result<()> {
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
    match connection.write_uint::<LittleEndian>(ret_val, 8) {
        Ok(()) => todo!(),
        Err(_) => todo!(),
    }
}

fn handle_call(connection: &mut dyn ReadWrite) -> Result<()> {
    let buff = read_msg_buffer(connection);
    let (cmd, _) = serde_json_core::de::from_slice::<CallCmd>(buff.as_slice()).unwrap();

    return make_call(
        connection,
        cmd.address as *const c_void,
        cmd.parameters.to_vec(),
    );
}

pub fn handle_client<RW: ReadWrite>(connection: &mut RW) -> Result<()> {
    loop {
        // match connection.read_u32::<LittleEndian>(){
        //     Ok(()) => todo!(),
        //     Err(err) => err.kind()
        // }

        // TODO: remove unwrap
        let res = match FromPrimitive::from_u32(connection.read_u32::<LittleEndian>().unwrap()) {
            Some(CMD::READ) => handle_read(connection),
            Some(CMD::WRITE) => handle_write(connection),
            Some(CMD::CALL) => handle_call(connection),
            None => todo!(),
        };
        match res {
            Ok(()) => continue,
            Err(_) => todo!()//handle_error(err, &mut *connection)?,
        }
    }
}
pub trait Hal<RW: ReadWrite>{
    fn init_connection(&self) -> Result<Box<RW>>;
    fn handle_error(&self ,_err: anyhow::Error, _connection: &mut RW) -> Result<()>;
}
pub fn run<RW: ReadWrite, T: Hal<RW>>(hal: &T) {
    loop {
        let mut connection = hal.init_connection().unwrap();
        match handle_client(&mut *connection) {
            Ok(()) => return,
            Err(err) => match hal.handle_error(err, &mut *connection) {
                Ok(()) => continue,
                Err(_) => todo!(),
            },
        };
    }
}
