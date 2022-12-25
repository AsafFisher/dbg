#![feature(slice_pattern)]
#![feature(type_alias_impl_trait)]
#![feature(inherent_associated_types)]
#![feature(core_intrinsics)]
#![no_std]
mod arch;
mod comm;
mod hooks;

extern crate alloc;
extern crate base64;
use crate::hooks::{DetourHook, DynamicTrampoline};
use alloc::string::String;
use alloc::{string::ToString, vec::Vec};
use arch::hook;
pub use base64::{decode, encode};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use comm::message::{
    read_msg_buffer, CallCmd, InstallHookCmd, ReadCmd, Response, ResponseStatus, WriteCmd,
};
use core::ffi::c_void;
use core2::io::Read;
use core2::io::Write;
use hal::{Connection, Hal};
use hooks::interactive_hook::initialize_interactive_hook;
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
    DISCONNECT = 3,
    SHUTDOWN = 4,
    INSTALL_HOOK = 5,
    TOGGLE_HOOK = 6,
}

struct Engine;
impl Engine {
    pub fn run() {
        loop {
            Hal::print("waiting for connection\n");
            let mut connection = Hal::init_connection(None).unwrap();
            match Self::handle_client(&mut *connection) {
                Ok(should_shut_down) => {
                    if should_shut_down {
                        Hal::print("Shutting down\n");
                        return;
                    }
                }
                Err(()) => continue,
            };
        }
    }

    fn handle_write(message: &[u8]) -> Result<Response, String> {
        let write_cmd: WriteCmd = minicbor::decode(message).unwrap();
        unsafe {
            let slice =
                core::slice::from_raw_parts_mut(write_cmd.address as *mut u8, write_cmd.buff.len());
            // copy byte by byte from buff to slice
            slice.copy_from_slice(write_cmd.buff.as_slice());
        }

        Ok(Response::BytesWritten {
            written: write_cmd.buff.len() as u64,
        })
    }

    fn handle_read(message: &[u8]) -> Result<Response, String> {
        let read_cmd: ReadCmd = minicbor::decode(message).unwrap();
        unsafe {
            let mut read_buff = alloc::vec::Vec::<u8>::new();
            // write_all might not be able to check that the buffer has the correct address.
            match read_buff.write_all(core::slice::from_raw_parts(
                read_cmd.address as *mut u8,
                read_cmd.size as usize,
            )) {
                Ok(_) => Ok(Response::BytesRead {
                    buff: minicbor::bytes::ByteVec::from(read_buff),
                }),
                // TODO: make sure this is ok
                Err(_) => Err("Error reading from address".to_string()),
            }
        }
    }

    fn make_call(ptr: *const c_void, mut argunments: Vec<u64>) -> Result<u64, String> {
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
            return Err("Too many params".to_string());
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

    fn handle_call(message: &[u8]) -> Result<Response, String> {
        let call_cmd: CallCmd = minicbor::decode(message).unwrap();

        let ret_val = Self::make_call(call_cmd.address as *const c_void, call_cmd.argunments);
        if let Ok(ret_val) = ret_val {
            Ok(Response::FunctionExecuted { ret: ret_val })
        } else {
            Err("Error executing function".to_string())
        }
    }

    fn handle_hook(message: &[u8]) -> Result<Response, String> {
        let hook_cmd: InstallHookCmd = minicbor::decode(message).unwrap();
        initialize_interactive_hook(hook_cmd)?;
        Ok(Response::HookInstalled)
    }

    fn handle_toggle_hook(_message: &[u8]) -> Result<Response, String> {
        Err("not implemented".to_string())
    }

    pub fn handle_client(connection: &mut Connection) -> core::result::Result<bool, ()> {
        let mut should_stop = None;
        loop {
            if let Some(is_shutdown) = should_stop {
                return Ok(is_shutdown);
            }
            let code = match connection.read_u32::<LittleEndian>() {
                Ok(code) => code,
                Err(_err) => {
                    Hal::print("Restarting service\n");
                    return Err(());
                }
            };

            let message_slc = read_msg_buffer(connection);

            // TODO: remove unwrap
            let res = match FromPrimitive::from_u32(code) {
                Some(CMD::READ) => Self::handle_read(message_slc.as_slice()),
                Some(CMD::WRITE) => Self::handle_write(message_slc.as_slice()),
                Some(CMD::CALL) => Self::handle_call(message_slc.as_slice()),
                Some(CMD::INSTALL_HOOK) => Self::handle_hook(message_slc.as_slice()),
                Some(CMD::TOGGLE_HOOK) => Self::handle_toggle_hook(message_slc.as_slice()),
                Some(CMD::DISCONNECT) => {
                    should_stop = Some(false);
                    Ok(Response::Disconnecting)
                }
                Some(CMD::SHUTDOWN) => {
                    should_stop = Some(true);
                    Ok(Response::Shutdown)
                }
                None => todo!(),
            };

            //TODO: FINISH
            let res = match res {
                Ok(response) => ResponseStatus::Success { response },
                Err(err) => {
                    // Error type import
                    ResponseStatus::Error { message: err }
                } //handle_error(err, &mut *connection)?,
            };
            let mut res_buf = alloc::vec::Vec::<u8>::new();
            minicbor::encode(res, &mut res_buf).unwrap();
            connection
                .write_u64::<LittleEndian>(res_buf.len() as u64)
                .unwrap();
            connection.write(res_buf.as_slice()).unwrap();
        }
    }
}

pub fn run() {
    Engine::run();
}
