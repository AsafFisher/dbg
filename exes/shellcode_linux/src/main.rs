// #![feature(lang_items)]
// #![feature(alloc_error_handler)]
// #![no_std]
// #![no_main]
// // extern crate rlibc;
// // use hal_shellcode_linuxgdb::hal_run;
// // const hello: &str = "hello_world";
// // // Move main to core, and make hal a lib if shellcode - use hal::shellcode etc
// // extern "C" fn pr(a: usize, b: usize, c: u8) -> u64 // Address is printed
// // {
// //     //println!("hello {:?} {:?} {:?}", a, b, c);
// //     return 4;
// // }

// #[panic_handler]
// fn panic(_: &core::panic::PanicInfo) -> ! {
//     loop {}
// }

// #[no_mangle]
// fn main(){
//     //println!("func: {:p}", pr as extern "C" fn(usize, usize, u8) -> _);
//     //println!("Const: {:p}", hello);
//     // unsafe{
//     //            hal_run();
//     // }
// }

// // #[lang = "eh_personality"] 
// // extern fn eh_personality() {}
// // //#[lang = "panic_fmt"] fn panic_fmt() -> ! { loop {} }
// // #[alloc_error_handler]
// // fn my_example_handler(layout: core::alloc::Layout) -> ! {
// //     panic!("memory allocation of {} bytes failed", layout.size())
// // }

#![no_std]
#![no_main]
#![feature(asm)]
#![feature(default_alloc_error_handler)]
//extern crate rlibc;
extern crate compiler_builtins;
use hal_shellcode_linuxgdb::hal_run;
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe {
            asm!("int 3");
        }
    }
}

const SYS_WRITE: usize = 1;
const SYS_EXIT: usize = 60;
const STDOUT: usize = 1;
static MESSAGE: &str = "hello world\n";

unsafe fn syscall1(syscall: usize, arg1: usize) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        in("rax") syscall,
        in("rdi") arg1,
        out("rcx") _,
        out("r11") _,
        lateout("rax") ret,
        options(nostack),
    );
    ret
}

unsafe fn syscall3(syscall: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        in("rax") syscall,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        out("rcx") _,
        out("r11") _,
        lateout("rax") ret,
        options(nostack),
    );
    ret
}

#[no_mangle]
fn _start() {
    unsafe {
        syscall3(
            SYS_WRITE,
            STDOUT,
            MESSAGE.as_ptr() as usize,
            MESSAGE.len(),
        );
        hal_run();
        syscall1(SYS_EXIT, 0)
    };
}