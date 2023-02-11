use byteorder::LittleEndian;
use byteorder::{ReadBytesExt, WriteBytesExt};
use libcore::comm::message::{
    CallCmd, HookPostCall, HookPostCallResponse, HookPreCallResponse, HookPrecall, InstallHookCmd,
    ReadCmd, Response, ResponseStatus, ToggleHookCmd, WriteCmd, CMD,
};
use minicbor;
use num_traits::ToPrimitive;
use pyo3::types::PyBytes;
use pyo3::{exceptions, prelude::*};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::thread::sleep;
use std::time::Duration;

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

fn send_msg_buffer(connection: &mut TcpStream, buff: &[u8]) {
    // TODO: remove unwrap
    connection
        .write_u64::<LittleEndian>(buff.len() as u64)
        .unwrap();
    // TODO: remove unwrap
    connection.write_all(buff).unwrap();
}

fn send_msg(connection: &mut TcpStream, msg_type: CMD, msg_data: Option<&[u8]>) {
    // TODO: remove unwrap
    connection
        .write_u32::<LittleEndian>(ToPrimitive::to_u32(&msg_type).unwrap())
        .unwrap();
    if let Some(msg_data) = msg_data {
        send_msg_buffer(connection, msg_data);
    }
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

/// Describes the way we want to approach with the retries
enum RetryPolicy {
    // Never stop retrying
    NoneStop,

    // Do N retries
    Retry(usize),
}

/// Attempt to connect to `address` for `retries` amount of times and between each try
/// wait `interval` amount of time
fn try_connect<A: ToSocketAddrs>(
    address: A,
    retries: RetryPolicy,
    interval: Duration,
) -> PyResult<TcpStream> {
    let mut try_count = 0;
    loop {
        match TcpStream::connect(&address) {
            Ok(stream) => return Ok(stream),
            Err(err) => match err.kind() {
                std::io::ErrorKind::ConnectionRefused => match retries {
                    RetryPolicy::NoneStop => continue,
                    RetryPolicy::Retry(retries) => {
                        if try_count == retries {
                            return Err(exceptions::PyConnectionError::new_err(err.to_string()));
                        }
                        try_count += 1;
                    }
                },
                _ => return Err(exceptions::PyConnectionError::new_err(err.to_string())),
            },
        }
        sleep(interval);
    }
}

/// HookController is responsible for implementing all the low level API endpoint with the debugger hook component.
///
/// These functions MUST not be used directly to communicate with the debugger, the order of which these functions are used
/// is important for the flow of the debugger.
///
/// # Example
///
/// ```python
/// # Initialize a connection with the debugger
/// controller = HookController("127.0.0.1:5555")
///
/// # Wait for the hook to be called and fetch the arguments of the call
/// args = controller.recv_precall_args()
///
/// # Call the original function with `args`, alternativly `skip_original_function` could have been used, but NOT both.
/// controller.call_original_with_args(args)
///
/// # Will set the return value of the function to the caller of the hooked function to -1.
/// controller.postcall_set_retval(-1)
/// ```
///
#[pyclass]
struct HookController {
    conn: TcpStream,
}
#[pymethods]
impl HookController {
    #[new]
    #[args(address, retries = "5", interval = "1.0")]
    pub fn new(py: Python, address: String, retries: usize, interval: f64) -> PyResult<Self> {
        py.allow_threads(|| {
            Ok(Self {
                conn: try_connect(
                    address,
                    RetryPolicy::Retry(retries),
                    Duration::from_secs_f64(interval),
                )?,
            })
        })
    }
    pub fn __enter__(slf: PyRef<Self>) -> PyResult<PyRef<Self>> {
        Ok(slf)
    }
    pub fn __exit__(
        slf: PyRefMut<Self>,
        _exc_type: &crate::PyAny,
        _exc_value: &crate::PyAny,
        _traceback: &crate::PyAny,
    ) {
        // TODO: Think what happen on this scenerio in the debugger
        slf.conn.shutdown(std::net::Shutdown::Both).unwrap();
    }

    /// Returns the arguments of the hooked function before it was called
    ///
    /// Must be used first
    fn recv_precall_args(&mut self, py: Python) -> PyResult<Vec<u64>> {
        py.allow_threads(|| {
            let hook_params: HookPrecall =
                minicbor::decode(&read_msg_buffer(&mut self.conn)).unwrap();
            Ok(hook_params.hook_arguments)
        })
    }

    /// Call the original function with the `arguments` that we choose
    ///
    /// Must be called second. Alternativly `skip_original_function` can be used to not call the original function
    fn call_original_with_args(&mut self, py: Python, arguments: Vec<u64>) -> PyResult<u64> {
        py.allow_threads(|| {
            let mut buff = std::vec::Vec::<u8>::new();
            minicbor::encode(
                HookPreCallResponse {
                    hook_arguments: arguments,
                    call_original: true,
                },
                &mut buff,
            )
            .unwrap();
            send_msg_buffer(&mut self.conn, &buff);
            let postcall: HookPostCall =
                minicbor::decode(&read_msg_buffer(&mut self.conn)).unwrap();
            Ok(postcall.hook_return_value)
        })
    }

    /// Dont call the original function, just keep going for the returnvalue stage.
    ///
    /// Must be called second. An alternative to call_original_with_args.
    fn skip_original_function(&mut self, py: Python) {
        py.allow_threads(|| {
            let mut buff = std::vec::Vec::<u8>::new();
            minicbor::encode(
                HookPreCallResponse {
                    hook_arguments: std::vec::Vec::new(),
                    call_original: false,
                },
                &mut buff,
            )
            .unwrap();
            send_msg_buffer(&mut self.conn, &buff);
        })
    }
    /// Set the new retval that will be returned from the hook instead of the original retval.
    ///
    /// Must be called last.
    fn postcall_set_retval(&mut self, py: Python, retval: u64) {
        py.allow_threads(|| {
            let mut buff = std::vec::Vec::<u8>::new();
            minicbor::encode(
                HookPostCallResponse {
                    hook_return_value: retval,
                },
                &mut buff,
            )
            .unwrap();
            send_msg_buffer(&mut self.conn, &buff);
        })
    }
}
#[pyclass]
struct DebugController {
    conn: TcpStream,
}

/// DebugController is responsible for implementing all the low level API endpoint with the debugger.
///
/// These functions should not be used directly to communicate with the debugger,
/// the user would want to use the python wrappers.
///
/// # Example
///
/// ```python
/// # Initialize a connection with the debugger
/// controller = DebugController("127.0.0.1:1234")
///
/// # Call the address 0x123456 as function with the parameters [1,2,3,4]
/// controller.call(0x123456, [1,2,3,4])
/// ```
///
#[pymethods]
impl DebugController {
    #[new]
    pub fn new(address: String) -> PyResult<Self> {
        match TcpStream::connect(address) {
            Ok(stream) => Ok(DebugController { conn: stream }),
            Err(err) => Err(exceptions::PyConnectionError::new_err(err.to_string())),
        }
    }

    /// This function tells the debugger to make a call to `address` with `arguments` as parameters
    ///
    /// # Example
    ///
    /// ```python
    /// # Initialize a connection with the debugger
    /// controller = DebugController("127.0.0.1:1234")
    ///
    /// # Call the address 0x123456 as function with the parameters [1,2,3,4]
    /// controller.call(0x123456, [1,2,3,4])
    /// ```
    pub fn call(&mut self, py: Python, address: u64, arguments: Vec<u64>) -> PyResult<u64> {
        py.allow_threads(|| {
            let mut send_buff = std::vec::Vec::<u8>::new();
            minicbor::encode(
                CallCmd {
                    address: address,
                    argunments: arguments,
                },
                &mut send_buff,
            )
            .unwrap();
            // TODO: remove unwrap
            send_msg(&mut self.conn, CMD::Call, Some(&send_buff));
            match get_response_if_success(&mut self.conn) {
                Ok(res) => match res {
                    Response::FunctionExecuted { ret } => Ok(ret),
                    _ => Err(exceptions::PyException::new_err("Got the wrong response!")),
                },
                Err(err) => Err(exceptions::PyException::new_err(err)),
            }
        })
    }

    /// This function tells the debugger to make a read `size` amount of bytes from `address`
    ///
    /// # Example
    ///
    /// ```python
    /// # Initialize a connection with the debugger
    /// controller = DebugController("127.0.0.1:1234")
    ///
    /// # Read 4 bytes from the address 0x123456
    /// assert controller.read(0x123456, 4) == b"\x00\x00\x00\x00"
    /// ```
    pub fn read(&mut self, py: Python, address: u64, size: u64) -> PyResult<Py<PyBytes>> {
        let mut send_buff = std::vec::Vec::<u8>::new();
        minicbor::encode(
            ReadCmd {
                address: address,
                size: size,
            },
            &mut send_buff,
        )
        .unwrap();
        // TODO: remove unwrap
        send_msg(&mut self.conn, CMD::Read, Some(&send_buff));
        match get_response_if_success(&mut self.conn) {
            Ok(res) => match res {
                Response::BytesRead { buff } => Ok(PyBytes::new(py, &buff).into()),
                _ => Err(exceptions::PyException::new_err("Got the wrong response!")),
            },
            Err(err) => Err(exceptions::PyException::new_err(err)),
        }
    }

    /// This function tells the debugger to make a write `buff` to `address` on the remote process.
    ///
    /// # Example
    ///
    /// ```python
    /// # Initialize a connection with the debugger
    /// controller = DebugController("127.0.0.1:1234")
    ///
    /// # Write b"\x00\x00\x00\x00" bytes to the address 0x123456
    /// assert controller.write(0x123456, b"\x00\x00\x00\x00") == 4
    /// ```
    pub fn write(&mut self, address: u64, buff: Vec<u8>) -> PyResult<u64> {
        let mut send_buff = std::vec::Vec::<u8>::new();
        minicbor::encode(
            WriteCmd {
                address: address,
                buff: buff.into(),
            },
            &mut send_buff,
        )
        .unwrap();
        // TODO: remove unwrap
        send_msg(&mut self.conn, CMD::Write, Some(&send_buff));
        match get_response_if_success(&mut self.conn) {
            Ok(res) => match res {
                Response::BytesWritten { written } => Ok(written),
                _ => Err(exceptions::PyException::new_err("Got the wrong response!")),
            },
            Err(err) => Err(exceptions::PyException::new_err(err)),
        }
    }

    /// This function tells the debugger to make a hook on `address`, and listen on `port` for client to connect
    /// in order to get the hook callbacks
    ///
    /// `prefix_size` is the size of the minimum instructions required to overwrite in order to replace them with the hook
    /// and save them in the trampoline.
    /// TODO: Make an example
    ///
    /// # Example
    ///
    /// ```python
    /// # Initialize a connection with the debugger
    /// controller = DebugController("127.0.0.1:1234")
    ///
    /// # Report to the debugger to create a hook on 0x123456 and listen on port 5555
    /// controller.hook(0x123456, 0xe, 5555)
    /// ```
    pub fn hook(&mut self, py: Python, address: u64, prefix_size: u64, port: u16) -> PyResult<()> {
        py.allow_threads(|| {
            let mut send_buff = std::vec::Vec::<u8>::new();
            minicbor::encode(
                InstallHookCmd {
                    address: address,
                    prefix_size: prefix_size,
                    port: port as u64,
                },
                &mut send_buff,
            )
            .unwrap();
            // TODO: remove unwrap
            send_msg(&mut self.conn, CMD::InstallHook, Some(&send_buff));
            match get_response_if_success(&mut self.conn) {
                Ok(res) => match res {
                    Response::HookInstalled => Ok(()),
                    _ => Err(exceptions::PyException::new_err("Got the wrong response!")),
                },
                Err(err) => Err(exceptions::PyException::new_err(err)),
            }
        })
    }

    /// This function tells the debugger to enable or disable a hook for `address`
    ///
    /// # Example
    ///
    /// ```python
    /// # Initialize a connection with the debugger
    /// controller = DebugController("127.0.0.1:1234")
    ///
    /// # Tell the debugger to disable the hook for 0x123456
    /// controller.hook_toggle(0x123456, False)
    /// ```
    pub fn hook_toggle(&mut self, py: Python, address: u64, enabled: bool) -> PyResult<()> {
        py.allow_threads(|| {
            let mut send_buff = std::vec::Vec::<u8>::new();
            minicbor::encode(
                ToggleHookCmd {
                    address: address,
                    enabled: enabled,
                },
                &mut send_buff,
            )
            .unwrap();
            // TODO: remove unwrap
            send_msg(&mut self.conn, CMD::ToggleHook, Some(&send_buff));
            match get_response_if_success(&mut self.conn) {
                Ok(res) => match res {
                    Response::HookToggled => Ok(()),
                    _ => Err(exceptions::PyException::new_err("Got the wrong response!")),
                },
                Err(err) => Err(exceptions::PyException::new_err(err)),
            }
        })
    }

    /// Disconnects from the debugger.
    ///
    /// The debugger will wait for a new connection on the same port.
    ///
    /// # Example
    ///
    /// ```python
    /// # Initialize a connection with the debugger
    /// controller = DebugController("127.0.0.1:1234")
    ///
    /// # Tell the debugger we are disconnecting.
    /// controller.disconnect()
    ///
    /// # Connect again to the debugger
    /// controller = DebugController("127.0.0.1:1234")
    /// ```
    pub fn disconnect(&mut self) -> PyResult<()> {
        // TODO: remove unwrap
        send_msg(&mut self.conn, CMD::Disconnect, None);
        match get_response_if_success(&mut self.conn) {
            Ok(res) => match res {
                Response::Disconnecting => Ok(()),
                _ => Err(exceptions::PyException::new_err("Got the wrong response!")),
            },
            Err(err) => Err(exceptions::PyException::new_err(err)),
        }
    }

    /// Shutdown the debugger.
    ///
    /// The debugger will return flow to whatever was before, or exit.
    ///
    /// # Example
    ///
    /// ```python
    /// # Initialize a connection with the debugger
    /// controller = DebugController("127.0.0.1:1234")
    ///
    /// # Tell the debugger to die.
    /// controller.shutdown()
    ///
    /// # Will fail
    /// controller = DebugController("127.0.0.1:1234")
    /// ```
    pub fn shutdown(&mut self) -> PyResult<()> {
        // TODO: remove unwrap
        send_msg(&mut self.conn, CMD::Shutdown, None);
        match get_response_if_success(&mut self.conn) {
            Ok(res) => match res {
                Response::Shutdown => Ok(()),
                _ => Err(exceptions::PyException::new_err("Got the wrong response!")),
            },
            Err(err) => Err(exceptions::PyException::new_err(err)),
        }
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn debugger_core(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<DebugController>()?;
    m.add_class::<HookController>()?;
    Ok(())
}
