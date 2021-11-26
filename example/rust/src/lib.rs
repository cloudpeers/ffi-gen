#![feature(asm)]
#![feature(asm_experimental_arch)]

ffi_gen_macro::ffi_gen!("example/rust/api.rsh");

#[cfg(target_family = "wasm")]
extern "C" {
    fn __console_log(ptr: isize, len: usize);
    fn call_function0(idx: i32);
    fn call_function1(idx: i32, arg: i32);
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

#[cfg(target_family = "wasm")]
#[no_mangle]
pub extern "C" fn invoke_callback0(idx: i32) {
    unsafe { call_function0(idx) };
}

#[cfg(target_family = "wasm")]
#[no_mangle]
pub extern "C" fn invoke_callback1(idx: i32, arg: i32) {
    unsafe { call_function1(idx, arg) };
}
