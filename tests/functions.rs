use ffi_gen::compile_pass;

compile_pass! {
    no_args_no_ret,
    "hello_world fn();",
    ( pub fn hello_world() {} ),
    ( __hello_world(); ),
    ( api.hello_world(); ),
    ( api.hello_world(); ),
}

/*
#[test]
fn no_args_ret_u8() {
    compile_pass(
        "hello_world fn() -> u8;",
        quote! {
            pub fn hello_world() -> u8 {
                42
            }

            fn main() {
                assert_eq!(__hello_world(), 42);
            }
        },
    )
    .unwrap();
}

#[test]
fn args_u8_ret_u8() {
    compile_pass(
        "hello_world fn(arg: u8) -> u8;",
        quote! {
            pub fn hello_world(arg: u8) -> u8 {
                arg
            }

            fn main() {
                assert_eq!(__hello_world(42), 42);
            }
        },
    )
    .unwrap();
}

#[test]
fn args_str_ret_usize() {
    compile_pass(
        "strlen fn(arg: &string) -> usize;",
        quote! {
            pub fn strlen(s: &str) -> usize {
                s.len()
            }

            fn main() {
                let s = "hello world";
                assert_eq!(__strlen(s.as_ptr() as isize, s.len()), 11);
            }
        },
    )
    .unwrap();
}

#[test]
fn args_string_ret_usize() {
    compile_pass(
        "strlen fn(arg: string) -> usize;",
        quote! {
            pub fn strlen(s: String) -> usize {
                s.len()
            }

            fn main() {
                use core::mem::ManuallyDrop;
                let s = ManuallyDrop::new("hello world".to_string());
                assert_eq!(__strlen(s.as_ptr() as isize, s.len(), s.capacity()), 11);
            }
        },
    )
    .unwrap();
}

#[test]
fn args_mut_string_string_no_ret() {
    compile_pass(
        "push_str fn(s: &mut string, s2: &string);",
        quote! {
            pub fn push_str(s: &mut String, s2: &str) {
                s.push_str(s2);
            }

            fn main() {
                let mut s = "hello".to_string();
                let s2 = " world";
                let mut alloc = Alloc {
                    ptr: s.as_mut_ptr() as _,
                    len: s.len() as _,
                    cap: s.capacity() as _,
                };
                __push_str(&mut alloc, s2.as_ptr() as _, s2.len());
                core::mem::forget(s);
                let s = unsafe { String::from_raw_parts(alloc.ptr as _, alloc.len as _, alloc.cap as _) };
                assert_eq!(s.as_str(), "hello world");
            }
        },
    )
    .unwrap();
}

#[test]
fn no_args_ret_string() {
    compile_pass(
        "make_string fn() -> string;",
        quote! {
            pub fn make_string() -> String {
                "hello world".to_string()
            }

            fn main() {
                let ret = __make_string();
                let s = unsafe { String::from_raw_parts(ret.ptr as _, ret.len as _, ret.cap as _) };
                assert_eq!(s.as_str(), "hello world");
            }
        }
    )
    .unwrap();
}

#[test]
fn args_str_ret_str() {
    compile_pass(
        "as_str fn(s: &string) -> &string;",
        quote! {
            pub fn as_str<'a>(s: &'a str) -> &'a str {
                s
            }

            fn main() {
                let s = "hello world";
                let ret = __as_str(s.as_ptr() as _, s.len());
                let slice = unsafe { core::slice::from_raw_parts(ret.ptr as _, ret.len as _) };
                let s2 = unsafe { std::str::from_utf8_unchecked(slice) };
                assert_eq!(s, s2);
            }
        },
    )
    .unwrap();
}*/
