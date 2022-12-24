#![feature(slice_pattern)]
#![feature(type_alias_impl_trait)]
#![feature(inherent_associated_types)]
#![feature(core_intrinsics)]
#![no_std]
mod arch;
mod hooks;

extern crate alloc;
extern crate base64;
use crate::hooks::{DetourHook, DynamicTrampoline};
use alloc::string::String;
use alloc::{string::ToString, vec::Vec};
pub use base64::{decode, encode};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use core::ffi::c_void;
use core2::io::Read;
use core2::io::Write;
use hal::{Connection, Hal};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::Deserialize;
use static_alloc::Bump;

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
enum Response {
    #[n(0)]
    BytesRead {
        #[n(0)]
        buff: minicbor::bytes::ByteVec,
    },
    #[n(1)]
    BytesWritten {
        #[n(0)]
        written: u64,
    },
    #[n(2)]
    FunctionExecuted {
        #[n(0)]
        ret: u64,
    },
    #[n(3)]
    Disconnecting,
    #[n(4)]
    Shutdown,
    #[n(5)]
    HookInstalled,
}
#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
enum ResponseStatus {
    #[n(0)]
    Success {
        #[n(0)]
        response: Response,
    },
    #[n(1)]
    Error {
        #[n(0)]
        message: String,
    },
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]

struct WriteCmd {
    #[n(0)]
    address: u64,
    #[n(1)]
    buff: minicbor::bytes::ByteVec,
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
struct ReadCmd {
    #[n(0)]
    address: u64,
    #[n(1)]
    size: u64,
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
struct CallCmd {
    #[n(0)]
    address: u64,
    #[n(1)]
    argunments: Vec<u64>,
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
struct InstallHookCmd {
    // Address to hook
    #[n(0)]
    address: u64,

    // Amount of bytes that will be needed
    #[n(1)]
    prefix_size: u64,

    // The port requested
    #[n(2)]
    port: u64,
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
struct ToggleHookCmd {
    #[n(0)]
    enabled: bool,
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
struct HookPrecall {
    #[n(0)]
    hook_arguments: Vec<u64>,
}
#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]

struct HookPreCallResponse {
    #[n(0)]
    hook_arguments: Vec<u64>,
    #[n(1)]
    call_original: bool,
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
struct HookPostCall {
    #[n(0)]
    hook_return_value: u64,
}
#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
struct HookPostCallResponse {
    #[n(0)]
    hook_return_value: u64,
}

#[global_allocator]
static A: Bump<[u8; 1 << 16]> = Bump::uninit();

#[cfg(feature = "linux_um")]
static mut HOOK_LIST: Vec<(DetourHook<DynamicTrampoline>, Connection)> = Vec::new();

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
    fn read_msg_buffer(connection: &mut Connection) -> Vec<u8> {
        let msg_size = connection.read_u64::<LittleEndian>().unwrap();
        let mut buff = Vec::with_capacity(msg_size as usize);
        buff.resize(msg_size as usize, 0);
        // Unwrap - get rid
        connection.read_exact(buff.as_mut_slice()).unwrap();
        buff
    }
    fn write_msg_buffer(connection: &mut Connection, buff: &Vec<u8>) {
        connection
            .write_u64::<LittleEndian>(buff.len() as u64)
            .unwrap();
        connection.write(buff.as_slice()).unwrap();
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

    fn generic_call_hook_handler(
        a: usize,
        b: usize,
        c: usize,
        d: usize,
        e: usize,
        f: usize,
        g: usize,
        h: usize,
        i: usize,
        j: usize,
        k: usize,
        m: usize,
    ) -> usize {
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h, mut i, mut j, mut k, mut m) =
            (a, b, c, d, e, f, g, h, i, j, k, m);
        let hook = unsafe {
            HOOK_LIST
                .iter_mut()
                .find(|(hook, _conn)| hook.callback == Self::generic_call_hook_handler)
        };

        // Should be unique, maybe create a function that derives it from the hook address.
        let ret_val = match hook {
            Some((hook, conn)) => {
                let args = [
                    a as u64, b as u64, c as u64, d as u64, e as u64, f as u64, g as u64, h as u64,
                    i as u64, j as u64, k as u64, m as u64,
                ];
                let mut args_buff = alloc::vec::Vec::<u8>::new();

                // TODO: Think how to handle errors.
                minicbor::encode(
                    HookPrecall {
                        hook_arguments: args.to_vec(),
                    },
                    &mut args_buff,
                )
                .unwrap();
                // TODO: Move all the message handling logic to a Message struct.
                Self::write_msg_buffer(conn, &args_buff);

                let args = Self::read_msg_buffer(conn);
                let args: HookPreCallResponse = minicbor::decode(&args).unwrap();
                let mut return_value = 0;
                if args.call_original {
                    let args = args.hook_arguments;
                    (a, b, c, d, e, f, g, h, i, j, k, m) = (
                        *args.get(0).unwrap_or_else(|| &0) as usize,
                        *args.get(1).unwrap_or_else(|| &0) as usize,
                        *args.get(2).unwrap_or_else(|| &0) as usize,
                        *args.get(3).unwrap_or_else(|| &0) as usize,
                        *args.get(4).unwrap_or_else(|| &0) as usize,
                        *args.get(5).unwrap_or_else(|| &0) as usize,
                        *args.get(6).unwrap_or_else(|| &0) as usize,
                        *args.get(7).unwrap_or_else(|| &0) as usize,
                        *args.get(8).unwrap_or_else(|| &0) as usize,
                        *args.get(9).unwrap_or_else(|| &0) as usize,
                        *args.get(10).unwrap_or_else(|| &0) as usize,
                        *args.get(11).unwrap_or_else(|| &0) as usize,
                    );
                    return_value = hook.call_trampoline(a, b, c, d, e, f, g, h, i, j, k, m);
                    let mut return_value_buff = alloc::vec::Vec::<u8>::new();
                    minicbor::encode(
                        HookPostCall {
                            hook_return_value: return_value as u64,
                        },
                        &mut return_value_buff,
                    )
                    .unwrap();
                    Self::write_msg_buffer(conn, &return_value_buff);
                }

                let retval_raw = Self::read_msg_buffer(conn);
                let recved_ret_val: HookPostCallResponse = minicbor::decode(&retval_raw).unwrap();
                recved_ret_val.hook_return_value as usize
            }
            None => 0,
        };

        ret_val
    }
    fn handle_hook(message: &[u8]) -> Result<Response, String> {
        let hook_cmd: InstallHookCmd = minicbor::decode(message).unwrap();
        let conn = Hal::init_connection(Some(hook_cmd.port as u16)).unwrap();

        // Creating the hook
        let hook = DetourHook::new(
            unsafe { core::mem::transmute(hook_cmd.address) },
            Self::generic_call_hook_handler,
            hook_cmd.prefix_size as usize,
        )?;

        // Inserting the hook to a static mut global. Why?
        // Because there is no way to share the shellcode's state with other threads that are already running.
        // Yes, there is a race if a hook is enabled! Mutex needed.
        unsafe { HOOK_LIST.push((hook, *conn)) };
        let (hook, _conn) = unsafe { HOOK_LIST.last() }.unwrap();
        unsafe {
            hook.enable()?;
        }
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

            let message_slc = Self::read_msg_buffer(connection);

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
