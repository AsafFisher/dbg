use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use libc::c_void;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    panic,
};
use thiserror::Error;

// extern crate proc_macro;
// use proc_macro::TokenStream;

// #[proc_macro]
// pub fn make_calls(_item: TokenStream) -> TokenStream {
//     let arguments_str = "usize";
//     format!("0 => \\{let fn: fn({arguments}) = ptr\\}", arguments_str).parse().unwrap()
// //    "fn answer() -> u32 { 42 }".parse().unwrap()
// }

#[derive(FromPrimitive, Deserialize)]
pub enum CMD {
    READ,
    WRITE,
    CALL,
}

pub trait ReadWrite: Read + Write {}
impl<T: Read + Write + ?Sized> ReadWrite for T {}

// #[derive(Error, Debug, Serialize)]
// enum Error {
//     #[error("Tried to access an invalid address")]
//     BadAddress { address: usize },
// }

#[derive(Deserialize, Debug)]
pub struct ReadCmd {
    size: usize,
    address: usize,
}

#[derive(Deserialize, Debug)]
pub struct CallCmd {
    address: usize,
    parameters: Vec<usize>,
}

#[derive(Deserialize, Debug)]
pub struct WriteCmd {
    address: usize,
    buffer: Vec<u8>,
}

extern "Rust" {
    fn init_connection() -> Result<Box<dyn ReadWrite>>;
    fn handle_error(err: anyhow::Error, connection: &mut dyn ReadWrite) -> Result<()>;
}

fn read_msg_buffer(connection: &mut dyn ReadWrite) -> Vec<u8> {
    let mut buff = [0; std::mem::size_of::<usize>()];
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

    let cmd = bincode::deserialize::<WriteCmd>(&buff).unwrap();
    unsafe {
        let slice = std::slice::from_raw_parts_mut(cmd.address as *mut u8, cmd.buffer.len());
        slice.copy_from_slice(&cmd.buffer)
    }

    Ok(())
}

fn handle_read(connection: &mut dyn ReadWrite) -> Result<()> {
    let mut buff = [0; std::mem::size_of::<usize>()];
    connection.read_exact(&mut buff).unwrap();

    // ptr
    let mut buff1 = [0; std::mem::size_of::<usize>()];
    connection.read_exact(&mut buff1).unwrap();

    let raw_pointer = usize::from_ne_bytes(buff1) as *mut u8;
    let len = usize::from_ne_bytes(buff);
    unsafe {
        // write_all might not be able to check that the buffer has the correct address.
        connection.write_all(std::slice::from_raw_parts(raw_pointer, len))?
    }
    Ok(())
}

fn make_call(
    connection: &mut dyn ReadWrite,
    ptr: *const c_void,
    mut argunments: Vec<usize>,
) -> Result<()> {
    if argunments.len() >= 20 {
        println!("Too many parameters");
        return Err(anyhow::anyhow!("Too many parameters"));
    }
    unsafe {
        match argunments.len() {
            0 => {
                connection.write_uint::<LittleEndian>(
                    std::mem::transmute::<_, extern "C" fn() -> u64>(ptr)(),
                    8,
                )?;
            }
            1 => {
                connection.write_uint::<LittleEndian>(
                    std::mem::transmute::<_, extern "C" fn(usize) -> u64>(ptr)(
                        argunments.remove(0),
                    ),
                    8,
                )?;
            }
            2 => {
                connection.write_uint::<LittleEndian>(
                    std::mem::transmute::<_, extern "C" fn(usize, usize) -> u64>(ptr)(
                        argunments.remove(0),
                        argunments.remove(0),
                    ),
                    8,
                )?;
            }
            3 => {
                connection.write_uint::<LittleEndian>(
                    std::mem::transmute::<_, extern "C" fn(usize, usize, usize) -> u64>(ptr)(
                        argunments.remove(0),
                        argunments.remove(0),
                        argunments.remove(0),
                    ),
                    8,
                )?;
            }
            _ => panic!("wong"),
        }
    }
    Ok(())
    // make_calls!();
    // for x in 0..20{

    //     let arguments_str = "usize";
    //     format!("x => \\{let fn: fn({arguments}) = ptr\\}", arguments_str)
    // }
}

fn handle_call(connection: &mut dyn ReadWrite) -> Result<()> {
    let buff = read_msg_buffer(connection);
    let cmd = bincode::deserialize::<CallCmd>(&buff).unwrap();
    return make_call(connection, cmd.address as *const c_void, cmd.parameters);
}

pub fn handle_client(mut connection: Box<dyn ReadWrite + 'static>) -> Result<()> {
    loop {
        let res = match FromPrimitive::from_u32(connection.read_u32::<LittleEndian>().unwrap()) {
            Some(CMD::READ) => handle_read(&mut *connection),
            Some(CMD::WRITE) => handle_write(&mut *connection),
            Some(CMD::CALL) => handle_call(&mut *connection),
            None => todo!(),
        };
        match res {
            Ok(()) => continue,
            Err(err) => unsafe { handle_error(err, &mut *connection)? },
        }
    }
}

pub unsafe fn run() {
    let connection = init_connection().unwrap();
    match handle_client(connection) {
        Ok(()) => return,
        Err(err) => panic!("{:?}", err),
    };
}
