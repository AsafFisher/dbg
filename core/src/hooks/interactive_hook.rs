use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use hal::{Connection, Hal};

use crate::comm::message::{
    read_msg_buffer, write_msg_buffer, InstallHookCmd, ToggleHookCmd, UninstallHookCmd,
};

use super::{DetourHook, DynamicTrampoline};

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

pub struct InteractiveHooks {
    hooks: Vec<(DetourHook<DynamicTrampoline>, Connection)>,
}
#[cfg(feature = "linux_um")]
pub static mut G_INTERACTIVE_HOOK: InteractiveHooks = InteractiveHooks { hooks: Vec::new() };

impl InteractiveHooks {
    pub fn get_instance() -> &'static mut Self {
        unsafe { &mut G_INTERACTIVE_HOOK }
    }

    pub fn initialize_interactive_hook(&mut self, hook_cmd: InstallHookCmd) -> Result<(), String> {
        let conn = Hal::init_connection(Some(hook_cmd.port as u16)).unwrap();

        // Creating the hook
        let hook = DetourHook::<DynamicTrampoline>::new(
            unsafe { core::mem::transmute(hook_cmd.address) },
            generic_call_hook_handler,
            hook_cmd.prefix_size as usize,
        )?;

        // Inserting the hook to a static mut global. Why?
        // Because there is no way to share the shellcode's state with other threads that are already running.
        // Yes, there is a race if a hook is enabled! Mutex needed.
        let a: &mut Vec<(DetourHook<DynamicTrampoline>, Connection)> = self.hooks.as_mut();
        a.push((hook, conn));

        Ok(())
    }

    pub fn uninintialize_interactive_hook(
        &mut self,
        hook_cmd: UninstallHookCmd,
    ) -> Result<(), String> {
        if let Some((index, _)) = self
            .hooks
            .iter_mut()
            .enumerate()
            .find(|(_, (hook, _))| hook.target() as u64 == hook_cmd.address)
        {
            self.hooks.remove(index);
            Ok(())
        } else {
            Err("Hook was not found".to_string())
        }
    }

    pub fn toggle_interactive_hook(&mut self, hook_cmd: ToggleHookCmd) -> Result<(), String> {
        let hook = self
            .hooks
            .iter_mut()
            .find(|(hook, _conn)| hook.target() as u64 == hook_cmd.address);
        if let Some(hook) = hook {
            let (detour, _) = hook;

            // This will atomically write patch in the hook
            unsafe { detour.toggle(hook_cmd.enabled)? };
            Ok(())
        } else {
            Err("Cannot toggle hook, hook not found".to_string())
        }
    }
}

// We do not support recurrsion!
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
        G_INTERACTIVE_HOOK
            .hooks
            .iter_mut()
            .find(|(hook, _conn)| hook.callback == generic_call_hook_handler)
    };

    // Should be unique, maybe create a function that derives it from the hook address.
    // To deal with this, we need to add a type for each of the hook callback requests. So in the client side,
    // The python will handle differently new requests in the nested hook.
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
            write_msg_buffer(conn, &args_buff);
            let args = read_msg_buffer(conn);
            let args: HookPreCallResponse = minicbor::decode(&args).unwrap();

            // If client did not call the original hook function, we should not execute the original function
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

                // Call the actual function
                let return_value = hook.call_trampoline(a, b, c, d, e, f, g, h, i, j, k, m);

                // Report the return value to the client.
                let mut return_value_buff = alloc::vec::Vec::<u8>::new();
                minicbor::encode(
                    HookPostCall {
                        hook_return_value: return_value as u64,
                    },
                    &mut return_value_buff,
                )
                .unwrap();
                write_msg_buffer(conn, &return_value_buff);
            }

            // Read the return value that the client decided to return
            let retval_raw = read_msg_buffer(conn);
            let recved_ret_val: HookPostCallResponse = minicbor::decode(&retval_raw).unwrap();
            recved_ret_val.hook_return_value as usize
        }
        None => 0,
    };

    ret_val
}
