use libcore::comm::message::ResponseStatus;
use minicbor;
use pyo3::prelude::*;
/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}

/// A Python module implemented in Rust.
#[pymodule]
fn debugger_core(_py: Python, m: &PyModule) -> PyResult<()> {
    let resp: ResponseStatus = minicbor::decode(b"helloworldhahahah").unwrap();
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    Ok(())
}
