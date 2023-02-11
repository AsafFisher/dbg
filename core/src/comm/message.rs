use crate::hal::Connection;
use alloc::string::String;
use alloc::vec::Vec;
use byteorder::LittleEndian;
use byteorder::{ReadBytesExt, WriteBytesExt};
use core2::io::Read;
use core2::io::Write;
use minicbor;
use num_derive::{FromPrimitive, ToPrimitive};

#[derive(FromPrimitive, ToPrimitive, Clone)]
pub enum CMD {
    Read = 0,
    Write = 1,
    Call = 2,
    Disconnect = 3,
    Shutdown = 4,
    InstallHook = 5,
    ToggleHook = 6,
    UninstallHook = 7,
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
pub enum Response {
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
    #[n(6)]
    HookToggled,
    #[n(7)]
    HookUninstalled,
}
#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
pub enum ResponseStatus {
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

pub struct WriteCmd {
    #[n(0)]
    pub address: u64,
    #[n(1)]
    pub buff: minicbor::bytes::ByteVec,
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
pub struct ReadCmd {
    #[n(0)]
    pub address: u64,
    #[n(1)]
    pub size: u64,
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
pub struct CallCmd {
    #[n(0)]
    pub address: u64,
    #[n(1)]
    pub argunments: Vec<u64>,
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
pub struct InstallHookCmd {
    // Address to hook
    #[n(0)]
    pub address: u64,

    // Amount of bytes that will be needed
    #[n(1)]
    pub prefix_size: u64,

    // The port requested
    #[n(2)]
    pub port: u64,
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
pub struct UninstallHookCmd {
    // Address to hook
    #[n(0)]
    pub address: u64,
}

#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
pub struct ToggleHookCmd {
    // Address of hook to enable
    #[n(0)]
    pub address: u64,

    // Enable/Disable
    #[n(1)]
    pub enabled: bool,
}

//      mov rdi, 1
//      call 0x12345 <--  At this point just after the jump we want to
//                        send the arguments... For this we use
//                        HookPrecall

// This the format of the arguments when sent before the hook
#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
pub struct HookPrecall {
    #[n(0)]
    // Arguments for the function that we hooked on.
    pub hook_arguments: Vec<u64>,
}
#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]

// TODO: replace with enum
// This struct contains the argument state the debugger want.
// Its a response for the HookPrecall
pub struct HookPreCallResponse {
    #[n(0)]
    // Argument that the debugger want to set for the hooked function
    pub hook_arguments: Vec<u64>,
    #[n(1)]
    // Do we want to call the original function?
    // If set to true, the original function will be called with the supplied
    // `hook_arguments` as parameters
    // If set to false, `hook_arguments` will be ignored and the original function
    // will not be called resulting in execution of the next stage (HookPostCall)
    pub call_original: bool,
}

// This struct contains the return value of the original function
// we hooked on
#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
pub struct HookPostCall {
    #[n(0)]
    // The result value of the original function
    pub hook_return_value: u64,
}

// This struct contains the return value the debugger want to set
#[derive(Debug, minicbor::Decode, minicbor::Encode, PartialEq)]
pub struct HookPostCallResponse {
    #[n(0)]
    // The result value the debugger want to return instead of the original one.
    pub hook_return_value: u64,
}

pub fn read_msg_buffer(connection: &mut Connection) -> Vec<u8> {
    let msg_size = connection.read_u64::<LittleEndian>().unwrap();
    let mut buff = Vec::with_capacity(msg_size as usize);
    buff.resize(msg_size as usize, 0);
    // Unwrap - get rid
    connection.read_exact(buff.as_mut_slice()).unwrap();
    buff
}

pub fn write_msg_buffer(connection: &mut Connection, buff: &Vec<u8>) {
    connection
        .write_u64::<LittleEndian>(buff.len() as u64)
        .unwrap();
    connection.write(buff.as_slice()).unwrap();
}
