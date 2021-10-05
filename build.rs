fn main() {
    // Tell Cargo that if the given file changes, to rerun this build script.
    //println!("cargo:rustc-cdylib-link-arg=-Wl,-Bstatic");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-Rsydmbols.txt");
}

// [build]
// rustflags = [
//   "-C",
//   "link-args=-Wl,-Rsymbols.txt"
// ]
