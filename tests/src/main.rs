// as the shellcode is not in the `.text` section, we can't execute it as it
#![feature(core_intrinsics)]
#[cfg(test)]
mod tests {
    use core::slice;
    use inline_python::{python, Context};
    use mmap::{
        MapOption::{MapExecutable, MapReadable, MapWritable},
        MemoryMap,
    };
    use rstest::*;
    use serial_test::serial;
    use std::str;
    use std::{intrinsics::breakpoint, mem};
    use std::{
        ops::Deref,
        sync::mpsc::{channel, Receiver, Sender},
        thread,
    };

    const SHELLCODE: &[u8] = include_bytes!("../../text.data");
    const WORD: &str = "Hello world";
    fn fibbo(n: usize) -> usize {
        if n <= 1 {
            return 1;
        }
        return fibbo(n - 1) + fibbo(n - 2);
    }
    extern "C" fn power(a: usize, b: usize) -> usize {
        let mut d = a;
        for _ in 0..b - 1 {
            d = d * a;
        }
        d
    }

    fn spawn_communicate<T: Send + 'static, CF: Fn(Sender<T>) + Send + 'static>(
        child_process_func: CF,
    ) -> Receiver<T> {
        let (tx, rx) = channel();
        thread::spawn(move || {
            child_process_func(tx);
        });
        rx
    }

    fn generate_read_write_exec_page(data: &[u8]) -> MemoryMap {
        let mapped =
            MemoryMap::new(data.len(), &[MapReadable, MapWritable, MapExecutable]).unwrap();
        unsafe { std::ptr::copy(data.as_ptr(), mapped.data(), data.len()) };
        mapped
    }
    fn with_debugger_enabled(tx: Sender<usize>) {
        // copy the shellcode to the memory map
        let string_block = generate_read_write_exec_page(WORD.as_bytes());
        let shellcode_block = generate_read_write_exec_page(SHELLCODE);
        tx.send(string_block.data() as usize).unwrap();
        unsafe {
            let exec_shellcode: extern "C" fn(base_addr: usize) =
                mem::transmute(shellcode_block.data());
            exec_shellcode(shellcode_block.data() as usize);
        }
    }

    struct DebugerController {
        ctx: Context,
    }
    impl DebugerController {
        fn new(ip: &str, port: u16) -> DebugerController {
            DebugerController {
                ctx: python! {
                    import debugger
                    proc = debugger.RemoteProcess('ip, 'port)
                },
            }
        }
    }
    impl Drop for DebugerController {
        fn drop(&mut self) {
            self.ctx.run(python! {
                proc.shutdown()
            })
        }
    }
    impl Deref for DebugerController {
        type Target = Context;
        fn deref(&self) -> &Context {
            &self.ctx
        }
    }

    #[fixture]
    #[once]
    fn debugger_and_address() -> usize {
        let debugee_process_rx = spawn_communicate(with_debugger_enabled);
        debugee_process_rx
            .recv()
            .expect("Failed to get address from test process") as usize
    }
    #[fixture]
    #[once]
    fn debugger_ctrl(debugger_and_address: &usize) -> DebugerController {
        DebugerController::new("127.0.0.1", 12343)
    }
    #[rstest]
    #[serial]
    fn read_address(debugger_and_address: &usize, debugger_ctrl: &DebugerController) {
        let pointer = debugger_and_address;
        // Fuck it, check it before we read it.
        let expected =
            str::from_utf8(unsafe { slice::from_raw_parts(*pointer as *mut u8, WORD.len()) })
                .unwrap();
        debugger_ctrl.run(python! {
            addr = proc.leak('pointer);
            leaked_word = addr.read(len('WORD)).decode("utf-8")
        });
        assert_eq!(debugger_ctrl.get::<String>("leaked_word"), expected);
    }
    #[rstest]
    #[serial]
    fn write_address(debugger_and_address: &usize, debugger_ctrl: &DebugerController) {
        let expected_after_change = "fucko world";
        let word_to_write = "fuck";
        let pointer = debugger_and_address;
        debugger_ctrl.run(python! {
            addr = proc.leak('pointer)
            amount_written = addr.write('word_to_write.encode("utf-8"))
        });
        assert_eq!(
            debugger_ctrl.get::<usize>("amount_written"),
            word_to_write.len()
        );
    }
    #[rstest]
    #[serial]
    fn call_address(debugger_and_address: &usize, debugger_ctrl: &DebugerController) {
        let fibbo_nth = 5;
        let pointer = fibbo as *mut u8 as usize;
        debugger_ctrl.run(python! {
            addr = proc.leak('pointer)
            result = addr('fibbo_nth);
        });
        assert_eq!(debugger_ctrl.get::<usize>("result"), fibbo(fibbo_nth));
    }

    #[rstest]
    #[serial]
    fn hook_address_different_param_call_original(
        debugger_and_address: &usize,
        debugger_ctrl: &DebugerController,
    ) {
        // Fuck it, check it before we read it.
        let power_addr = power as *mut u8 as usize;
        let num1 = 5;
        let num2 = 2;
        let fake_num1 = 3;
        let fake_num2 = 4;
        debugger_ctrl.run(python! {
            def hook_func1(original_hook, x, y):
                assert x == 'num1, "Invalid Hook parameter"
                assert y == 'num2, "Invalid Hook parameter"
                return original_hook('fake_num1, 'fake_num2)
            addr = proc.leak('power_addr);
            hook = addr.hook(0xe, hook_func1)
            hook.enable()
        });
        assert_eq!(power(num1, num2), 3 * 3 * 3 * 3);
        debugger_ctrl.run(python! {
            hook.disable()
        });
        assert_eq!(power(num1, num2), 5 * 5);
    }

    #[rstest]
    #[serial]
    fn hook_address_no_original(debugger_and_address: &usize, debugger_ctrl: &DebugerController) {
        // TODO: Make sure calling hook twice will result in change of the hook_func
        let power_addr = power as *mut u8 as usize;
        let num1 = 5;
        let num2 = 2;
        debugger_ctrl.run(python! {
            def hook_func2(_, x, y):
                assert x == 'num1, "Invalid Hook parameter"
                assert y == 'num2, "Invalid Hook parameter"
                return x + y
            addr = proc.leak('power_addr)
            hook = addr.hook(0xe, hook_func2)
            hook.enable()
        });
        assert_eq!(power(num1, num2), num1 + num2);
        debugger_ctrl.run(python! {
            hook.disable()
        });
    }

    #[rstest]
    #[serial]
    fn hook_address_different_param_call_original_contextmanager(
        debugger_and_address: &usize,
        debugger_ctrl: &DebugerController,
    ) {
        // Fuck it, check it before we read it.
        let power_addr = power as *mut u8 as usize;
        let num1 = 5;
        let num2 = 2;
        let fake_num1 = 3;
        let fake_num2 = 4;
        debugger_ctrl.run(python! {
            def hook_func1(original_hook, x, y):
                assert x == 'num1, "Invalid Hook parameter"
                assert y == 'num2, "Invalid Hook parameter"
                return original_hook('fake_num1, 'fake_num2)
            addr = proc.leak('power_addr);
            hook = addr.hook(0xe, hook_func1)
            assert addr('num1, 'num2) == 'num1 ** 'num2
            with hook.enabled():
                assert addr('num1, 'num2) == 'fake_num1 ** 'fake_num2
            assert addr('num1, 'num2) == 'num1 ** 'num2
        });
        assert_eq!(power(num1, num2), num1.pow(num2 as u32));
    }

    #[rstest]
    #[serial]
    fn hook_address_enable_disable_many(
        debugger_and_address: &usize,
        debugger_ctrl: &DebugerController,
    ) {
        // Fuck it, check it before we read it.
        let power_addr = power as *mut u8 as usize;
        let num1 = 5;
        let num2 = 2;
        let fake_num1 = 3;
        let fake_num2 = 4;
        debugger_ctrl.run(python! {
            def hook_func1(original_hook, x, y):
                assert x == 'num1, "Invalid Hook parameter"
                assert y == 'num2, "Invalid Hook parameter"
                return original_hook('fake_num1, 'fake_num2)
            addr = proc.leak('power_addr);
            hook = addr.hook(0xe, hook_func1)
            assert addr('num1, 'num2) == 'num1 ** 'num2
            with hook.enabled():
                assert addr('num1, 'num2) == 'fake_num1 ** 'fake_num2
            assert addr('num1, 'num2) == 'num1 ** 'num2
            with hook.enabled():
                assert addr('num1, 'num2) == 'fake_num1 ** 'fake_num2
            assert addr('num1, 'num2) == 'num1 ** 'num2
            with hook.enabled():
                assert addr('num1, 'num2) == 'fake_num1 ** 'fake_num2
            assert addr('num1, 'num2) == 'num1 ** 'num2
            with hook.enabled():
                assert addr('num1, 'num2) == 'fake_num1 ** 'fake_num2
            assert addr('num1, 'num2) == 'num1 ** 'num2
            with hook.enabled():
                assert addr('num1, 'num2) == 'fake_num1 ** 'fake_num2
            assert addr('num1, 'num2) == 'num1 ** 'num2
            
            # Without contextlib
            hook.enable()
            assert addr('num1, 'num2) == 'fake_num1 ** 'fake_num2
            hook.disable()
            assert addr('num1, 'num2) == 'num1 ** 'num2
            hook.enable()
            assert addr('num1, 'num2) == 'fake_num1 ** 'fake_num2
            hook.disable()
            assert addr('num1, 'num2) == 'num1 ** 'num2
            hook.enable()
            assert addr('num1, 'num2) == 'fake_num1 ** 'fake_num2
            hook.disable()
            assert addr('num1, 'num2) == 'num1 ** 'num2
            hook.enable()
            assert addr('num1, 'num2) == 'fake_num1 ** 'fake_num2
            hook.disable()
            assert addr('num1, 'num2) == 'num1 ** 'num2

            # Double enable and double disable
            hook.enable()
            hook.enable()
            assert addr('num1, 'num2) == 'fake_num1 ** 'fake_num2
            hook.disable()
            hook.disable()
            assert addr('num1, 'num2) == 'num1 ** 'num2
            hook.enable()
            hook.enable()
            assert addr('num1, 'num2) == 'fake_num1 ** 'fake_num2
            hook.disable()
            hook.disable()
            assert addr('num1, 'num2) == 'num1 ** 'num2
        });
        assert_eq!(power(num1, num2), num1.pow(num2 as u32));
        
    }
}
