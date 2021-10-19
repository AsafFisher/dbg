mod bits;
use core::run;
const hello: &str = "hello_world";
// Move main to core, and make hal a lib if shellcode - use hal::shellcode etc
extern "C" fn pr(a: usize, b: usize, c: u8) -> u64 // Address is printed
{
    println!("hello {:?} {:?} {:?}", a, b, c);
    return 4;
}

fn main(){
    println!("func: {:p}", pr as extern "C" fn(usize, usize, u8) -> _);
    println!("Const: {:p}", hello);
    unsafe{
               run();
    }
}