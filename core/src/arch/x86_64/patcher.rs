use core::sync::atomic::AtomicU64;

use alloc::{
    slice,
    string::{String, ToString},
    vec::Vec,
};
use hal::Hal;

use super::branch::generate_relative_branch;

fn write_atomic_patch(dest: &[u8], src: &[u8], i: usize) {
    let dest_atomic = unsafe { &*(dest.as_ptr() as *const AtomicU64) };
    let mut to_write: [u8; 8] = [0; 8];
    if i == 7 {
        to_write.copy_from_slice(&src[..i + 1]);
    } else if i < 7 {
        // Do partial copy only happen if
        to_write[i + 1..].copy_from_slice(&dest[i + 1..]);
        to_write[..i + 1].copy_from_slice(&src[..i + 1]);
    }
    dest_atomic.store(
        u64::from_le_bytes(to_write),
        core::sync::atomic::Ordering::Relaxed,
    )
}

fn patch_and_unlock(dst_code: *const (), src: &[u8]) {
    // SAFTY:
    // bytes_to_write's size is the amount of bytes we want to overwrite.
    // The last iteration of the for loop will unlock the execution of the hooked function
    let to_patch: &mut [u8] = unsafe { slice::from_raw_parts_mut(dst_code as *mut u8, src.len()) };
    for i in (0..to_patch.len()).rev() {
        if i <= 7 {
            // The last write will delete the lock, So it needs to be atomic.
            write_atomic_patch(to_patch, src, i);
            break;
        }
        to_patch[i] = src[i];
    }
}

pub struct Patcher {
    pub target: *const (),
    bytes_to_write: Vec<u8>,
    original_bytes: Vec<u8>,
}

impl Patcher {
    pub fn new(
        target: *const (),
        patch: Vec<u8>,
        actual_patch_size: usize,
    ) -> Result<Patcher, String> {
        if patch.len() > actual_patch_size {
            Err("Hook patch is too big for given actual_patch_size".to_string())
        } else {
            Ok(Patcher {
                target: target,
                bytes_to_write: patch,
                original_bytes: unsafe {
                    slice::from_raw_parts(target as *const u8, actual_patch_size)
                }
                .to_vec(),
            })
        }
    }

    fn lock_function(&self) -> Result<(), String> {
        let func_start = unsafe { &*(self.target as *const AtomicU64) };
        // atomically lock the function
        let mut atomic_buffer = [0; 8];
        let lock = generate_relative_branch(self.target, self.target, false)?;
        let patch_size = lock.len();
        if patch_size > atomic_buffer.len() {
            return Err("Lock size is too big for atomic locking".to_string());
        }
        atomic_buffer[..patch_size].copy_from_slice(&lock);
        atomic_buffer[patch_size..].fill(0x90);
        // let tmp = atomic_buffer.len();
        // atomic_buffer[patch_size..core::cmp::min(self.original_bytes.len(), tmp)].fill(0x90);
        // atomic_buffer[self.original_bytes.len()..].copy_from_slice(
        //     &func_start
        //         .load(core::sync::atomic::Ordering::Relaxed)
        //         .to_le_bytes()[self.original_bytes.len()..],
        // );
        let patch_value = u64::from_le_bytes(atomic_buffer);
        func_start.store(patch_value, core::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    pub unsafe fn toggle_hook(&self, enabled: bool) -> Result<(), String> {
        unsafe {
            Hal::enable_write(slice::from_raw_parts_mut(
                self.target as *mut u8,
                self.bytes_to_write.len(),
            ))
        }?;
        self.lock_function()?;
        if enabled {
            // Enable hook
            patch_and_unlock(self.target, &self.bytes_to_write);
        } else {
            // Disable hook
            patch_and_unlock(self.target, &self.original_bytes);
        }

        unsafe {
            Hal::disable_write(slice::from_raw_parts_mut(
                self.target as *mut u8,
                self.bytes_to_write.len(),
            ))
        }?;
        Ok(())
    }

    pub unsafe fn enable(&self) -> Result<(), String> {
        self.toggle_hook(true)
    }
    pub unsafe fn disable(&self) -> Result<(), String> {
        self.toggle_hook(false)
    }
}
