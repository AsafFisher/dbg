// as the shellcode is not in the `.text` section, we can't execute it as it
#![feature(core_intrinsics)]
#[cfg(feature = "python_integrated_test")]
mod integrated_tests;
mod utils;
