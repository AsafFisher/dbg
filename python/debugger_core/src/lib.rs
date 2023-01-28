use byteorder::LittleEndian;
use byteorder::{ReadBytesExt, WriteBytesExt};
use libcore::comm::message::{Response, ResponseStatus, CMD};
use minicbor;
use num_traits::ToPrimitive;
use pyo3::{exceptions, prelude::*};
use std::io::{Read, Write};
use std::net::TcpStream;

// TODO: Make this function generic and use the same read_msg_buffer
// of the `core::comm::message` module
fn read_msg_buffer(connection: &mut TcpStream) -> Vec<u8> {
    let msg_size = connection.read_u64::<LittleEndian>().unwrap();
    let mut buff = Vec::with_capacity(msg_size as usize);
    buff.resize(msg_size as usize, 0);
    // Unwrap - get rid
    connection.read_exact(buff.as_mut_slice()).unwrap();
    buff
}

fn send_msg(connection: &mut TcpStream, msg_type: CMD, msg_data: Option<&[u8]>) {
    // TODO: remove unwrap
    connection
        .write_u32::<LittleEndian>(ToPrimitive::to_u32(&msg_type).unwrap())
        .unwrap();
    if let Some(msg_data) = msg_data {
        // TODO: remove unwrap
        connection.write_u64::<LittleEndian>(msg_data.len() as u64).unwrap();
        // TODO: remove unwrap
        connection.write_all(msg_data).unwrap();
    }
}

/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}

pub fn get_response_if_success(connection: &mut TcpStream) -> Result<Response, String> {
    let buffer = read_msg_buffer(connection);

    // Change unwrap to python exception
    let res: ResponseStatus = minicbor::decode(&buffer).unwrap();
    match res {
        ResponseStatus::Success { response } => Ok(response),

        // TODO: change to custom error
        ResponseStatus::Error { message } => Err(message),
    }
}

#[pyclass]
struct DebugController {
    conn: TcpStream,
}

#[pymethods]
impl DebugController {
    #[new]
    pub fn new(address: String) -> PyResult<Self> {
        match TcpStream::connect(address) {
            Ok(stream) => Ok(DebugController { conn: stream }),
            Err(err) => Err(exceptions::PyConnectionError::new_err(err.to_string())),
        }
    }

    pub fn disconnect(&mut self) -> PyResult<()> {
        // TODO: remove unwrap

        send_msg(&mut self.conn, CMD::Disconnect, None);
        match get_response_if_success(&mut self.conn) {
            Ok(res) => match res {
                Response::Shutdown => Ok(()),
                _ => Err(exceptions::PyException::new_err("Got the wrong response!")),
            },
            Err(err) => Err(exceptions::PyException::new_err(err))
        }
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn debugger_core(_py: Python, m: &PyModule) -> PyResult<()> {
    //let resp: ResponseStatus = minicbor::decode(b"helloworldhahahah").unwrap();
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    m.add_class::<DebugController>()?;
    Ok(())
}
