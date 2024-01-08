use std::env;

fn main() {
    if env::var("CARGO_FEATURE_INSTRUMENTED").is_ok() {
        println!("cargo:rustc-env=RUSTFLAGS=\"--emit=llvm-ir -Z llvm-plugins=/home/aoli/marionette/instrumentation/llvm/build/src/libInjectYieldPass.so -C passes=inject-yield\"");
    }
}