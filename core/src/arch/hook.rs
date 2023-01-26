use alloc::vec;
use alloc::{string::String, vec::Vec};

use super::x86_64::branch::{generate_absolute_branch, ABSOLUTE_BRANCH_SIZE};
use super::x86_64::patcher::Patcher;

pub trait Trampoline
where
    Self: Sized,
{
    fn build(original_bytes: &[u8], callback: *const ()) -> Result<Self, String>;
    fn as_ptr(&self) -> *const ();
    fn start_address(&self) -> *const ();

    // # of parameters should be dynamic in the future using macros.
    fn call(
        &self,
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
    ) -> usize;
}
pub struct DynamicTrampoline {
    // TODO: Pin
    slide: Vec<u8>,
}
const POINTER_SIZE: usize = core::mem::size_of::<usize>();
impl Trampoline for DynamicTrampoline {
    // Assumes that the call/jmp instruction is always the last instruction.
    // original_bytes holds the bytes of the target function that we copied to the trampoline
    fn build(original_bytes: &[u8], target: *const ()) -> Result<DynamicTrampoline, String> {
        // The size of the trampoline needs to be equals to the amount of bytes we copy + the jmp
        // instruction.
        let mut slide: Vec<u8> = vec![0x90; original_bytes.len() + ABSOLUTE_BRANCH_SIZE];
        slide[..original_bytes.len()].copy_from_slice(original_bytes);

        // This is the address we jump to from the trampoline, we just original_bytes amount of bytes
        // after the target's original address because we placed the hook at the beggining of the `target`
        let address_to_jump_from_trampoline = (target as usize + original_bytes.len()) as *const ();

        // Create the jmp instruction to the (target + hook size) fn from the trampoline
        slide[original_bytes.len()..].copy_from_slice(
            &generate_absolute_branch(address_to_jump_from_trampoline)?.as_slice(),
        );
        Ok(DynamicTrampoline { slide: slide })
    }
    fn as_ptr(&self) -> *const () {
        self.slide.as_ptr() as *const ()
    }
    fn start_address(&self) -> *const () {
        self.slide[POINTER_SIZE..].as_ptr() as *const ()
    }
    fn call(
        &self,
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
        let func: fn(
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
        ) -> usize = unsafe { core::mem::transmute(self.as_ptr()) };
        func(a, b, c, d, e, f, g, h, i, j, k, m)
    }
}

pub struct DetourHook<T>
where
    T: Trampoline,
{
    pub callback: fn(
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
    ) -> usize,
    pub trampoline: T,
    patcher: Patcher,
}

impl<T> DetourHook<T>
where
    T: Trampoline,
{
    pub fn new(
        target: fn(),
        callback: fn(
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
            usize,
        ) -> usize,
        actual_patch_size: usize,
    ) -> Result<Self, String> {
        // Build a jump to our callback function, it passes control to the client, registers + trampoline addr.
        // Client can decide wether he want to call the trampoline or not.
        let patch = generate_absolute_branch(callback as *const ())?;
        let patcher = Patcher::new(target as *const (), patch, actual_patch_size)?;
        let original_bytes: &[u8] = unsafe {
            core::slice::from_raw_parts(target as *const () as *const u8, actual_patch_size)
        };
        let tramp = T::build(original_bytes, target as *const ())?;

        Ok(DetourHook {
            callback: callback,
            trampoline: tramp,
            patcher: patcher,
        })
    }

    // Eanbles the hook
    #[allow(dead_code)]
    pub unsafe fn enable(&self) -> Result<(), String> {
        self.patcher.enable()
    }
    // Disable the hook
    #[allow(dead_code)]
    pub unsafe fn disable(&self) -> Result<(), String> {
        self.patcher.disable()
    }
    // Eanbles the hook
    pub unsafe fn toggle(&self, enabled: bool) -> Result<(), String> {
        self.patcher.toggle_hook(enabled)
    }

    pub fn target(&self) -> *const () {
        self.patcher.target
    }

    pub fn call_trampoline(
        &self,
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
        self.trampoline.call(a, b, c, d, e, f, g, h, i, j, k, m)
    }
}

impl<T: Trampoline> Drop for DetourHook<T> {
    fn drop(&mut self) {
        unsafe {
            // unwrap because drop does not return a result.
            // If this operation fails we are dead...
            self.disable().unwrap();
        }
    }
}
#[cfg(test)]
mod test {
    fn main() {}
}
