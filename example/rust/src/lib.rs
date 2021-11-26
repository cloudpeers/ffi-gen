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

#[repr(C)]
pub struct DartCObject {
    pub ty: DartCObjectType,
    pub value: DartCObjectValue,
}

#[repr(i32)]
pub enum DartCObjectType {
    DartNull,
    DartBool,
    DartInt32,
    DartInt64,
    DartDouble,
    DartString,
    DartArray,
    DartTypedData,
    DartExternalTypedData,
    DartSendPort,
    DartCapability,
    DartUnsupported,
    DartNumberOfTypes,
}

#[repr(C)]
pub union DartCObjectValue {
    pub as_bool: bool,
    pub as_int32: i32,
    pub as_int64: i64,
    pub as_double: f64,
    /*pub as_string: *mut c_char,
    pub as_send_port: DartNativeSendPort,
    pub as_capability: DartNativeCapability,
    pub as_array: DartNativeArray,
    pub as_typed_data: DartNativeTypedData,*/
    // some fields omitted
}

#[no_mangle]
pub unsafe extern "C" fn register_callback(port: i64, post_cobject: *const core::ffi::c_void) {
    println!("register_callback");
    let post_cobject: extern "C" fn(i64, *const core::ffi::c_void) = core::mem::transmute(post_cobject);
    let obj: DartCObject =  core::mem::zeroed();
    post_cobject(port, &obj as *const _ as *const _);
}
