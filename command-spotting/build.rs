//! Lets every binary of this package find the shared ONNX Runtime and sherpa-onnx libraries
//! that `sherpa-onnx-sys` copies into Cargo's profile folder: a program beside them, a test
//! binary one folder down in `deps/`.

fn main() {
    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/..");
}
