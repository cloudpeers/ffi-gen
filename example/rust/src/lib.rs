ffi_gen_macro::ffi_gen!("example/rust/api.rsh");

#[cfg(target_family = "wasm")]
extern "C" {
    fn __console_log(ptr: isize, len: usize);
}

fn log(msg: &str) {
    #[cfg(target_family = "wasm")]
    return unsafe { __console_log(msg.as_ptr() as _, msg.len()) };
    #[cfg(not(target_family = "wasm"))]
    println!("{}", msg);
}

pub fn hello_world() {
    log("hello world");
}
