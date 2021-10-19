fn main() {
    println!("cargo:rustc-link-arg=-nostdlib");
    println!("cargo:rustc-link-arg=-static");
    println!("cargo:rustc-link-arg=-T./shellcode.ld");
    println!("cargo:rerun-if-changed=./shellcode.ld");
    // relocation-model pic


    // print current directory
    println!("cargo:warning=Current directory: {}", std::env::current_dir().unwrap().display());
}