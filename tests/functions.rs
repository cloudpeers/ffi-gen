use ffi_gen::compile_pass;

compile_pass! {
    no_args_no_ret,
    "fn hello_world();",
    ( pub fn hello_world() {} ),
    ( __hello_world(); ),
    ( api.helloWorld(); ),
    ( api.hello_world(); ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        hello_world(): void;

        drop(): void;
    })

}

compile_pass! {
    no_args_ret_u8,
    "fn hello_world() -> u8;",
    (
        pub fn hello_world() -> u8 {
            42
        }
    ),
    ( assert_eq!(__hello_world(), 42); ),
    ( assert(api.helloWorld() == 42); ),
    ( assert.equal(api.hello_world(), 42); ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        hello_world(): number;

        drop(): void;
    })

}

compile_pass! {
    args_u8_ret_u8,
    "fn hello_world(arg: u8) -> u8;",
    (
        pub fn hello_world(arg: u8) -> u8 {
            arg
        }
    ),
    ( assert_eq!(__hello_world(42), 42); ),
    ( assert(api.helloWorld(42) == 42); ),
    ( assert.equal(api.hello_world(42), 42); ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        hello_world(arg: number): number;

        drop(): void;
    })
}

compile_pass! {
    args_bool_ret_bool,
    "fn hello_world(arg: bool) -> bool;",
    (
        pub fn hello_world(arg: bool) -> bool {
            arg
        }
    ),
    ( assert_eq!(__hello_world(1), 1); ),
    ( assert(api.helloWorld(true) == true); ),
    ( assert.strictEqual(api.hello_world(true), true); ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        hello_world(arg: boolean): boolean;

        drop(): void;
    })
}

compile_pass! {
    args_usize_ret_usize,
    "fn hello_world(arg: usize) -> usize;",
    (
        pub fn hello_world(arg: usize) -> usize {
            arg
        }
    ),
    ( assert_eq!(__hello_world(42), 42); ),
    ( assert(api.helloWorld(42) == 42); ),
    ( assert.equal(api.hello_world(42), 42); ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        hello_world(arg: number): number;

        drop(): void;
    })

}

compile_pass! {
    args_f64_ret_f64,
    "fn hello_world(arg: f64) -> f64;",
    (
        pub fn hello_world(arg: f64) -> f64 {
            arg
        }
    ),
    ( assert_eq!(__hello_world(42.24), 42.24); ),
    ( assert(api.helloWorld(42.24) == 42.24); ),
    ( assert.equal(api.hello_world(42.24), 42.24); ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        hello_world(arg: number): number;

        drop(): void;
    })
}

compile_pass! {
    args_str_ret_usize,
    "fn strlen(arg: &string) -> usize;",
    (
        pub fn strlen(s: &str) -> usize {
            s.len()
        }
    ),
    (
        let s = "hello world";
        assert_eq!(__strlen(s.as_ptr() as _, s.len() as _), 11);
        let s = "مرحبا بالعالم";
        assert_eq!(__strlen(s.as_ptr() as _, s.len() as _), 25);
    ),
    (
        assert(api.strlen("hello world") == 11);
        assert(api.strlen("مرحبا بالعالم") == 25);
    ),
    (
        assert.equal(api.strlen("hello world"), 11);
        assert.equal(api.strlen("مرحبا بالعالم"), 25);
    ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        strlen(arg: string): number;

        drop(): void;
    })
}

compile_pass! {
    args_string_ret_usize,
    "fn strlen(arg: string) -> usize;",
    (
        pub fn strlen(s: String) -> usize {
            s.len()
        }
    ),
    (
        use core::mem::ManuallyDrop;
        let s = ManuallyDrop::new("hello world".to_string());
        assert_eq!(__strlen(s.as_ptr() as _, s.len() as _, s.capacity() as _), 11);
    ),
    ( assert(api.strlen("hello world") == 11); ),
    ( assert.equal(api.strlen("hello world"), 11); ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        strlen(arg: string): number;

        drop(): void;
    })
}

compile_pass! {
    no_args_ret_string,
    "fn make_string() -> string;",
    (
        pub fn make_string() -> String {
            "hello world".to_string()
        }
    ),
    (
        let ret = __make_string();
        let s = unsafe { String::from_raw_parts(ret.ret0 as _, ret.ret1 as _, ret.ret2 as _) };
        assert_eq!(s.as_str(), "hello world");
    ),
    ( assert(api.makeString() == "hello world"); ),
    ( assert.equal(api.make_string(), "hello world"); ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        make_string(): string;

        drop(): void;
    })
}

compile_pass! {
    args_str_ret_str,
    "fn as_str(s: &string) -> &string;",
    (
        pub fn as_str<'a>(s: &'a str) -> &'a str {
            s
        }
    ),
    (
        let s = "hello world";
        let ret = __as_str(s.as_ptr() as _, s.len() as _);
        let slice = unsafe { core::slice::from_raw_parts(ret.ret0 as _, ret.ret1 as _) };
        let s2 = unsafe { std::str::from_utf8_unchecked(slice) };
        assert_eq!(s, s2);
    ),
    ( assert(api.asStr("hello world") == "hello world"); ),
    ( assert.equal(api.as_str("hello world"), "hello world"); ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        as_str(s: string): string;

        drop(): void;
    }
    )

}

compile_pass! {
    args_slice_u8_ret_vec_u8,
    "fn to_vec(b: &[u8]) -> Vec<u8>;",
    (
        pub fn to_vec(b: &[u8]) -> Vec<u8> {
            b.to_vec()
        }
    ),
    ( ),
    (
        assert(api.toVec([]).equals([]));
        assert(api.toVec([0, 1, 2, 3, 4]).equals([0, 1, 2, 3, 4]));
    ),
    (
        assert.deepEqual(api.to_vec([]), []);
        assert.deepEqual(api.to_vec([0, 1, 2, 3, 4]), [0, 1, 2, 3, 4]);
    ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        to_vec(b: Array<number>): Array<number>;

        drop(): void;
    })

}

compile_pass! {
    args_slice_u64_ret_vec_u64,
    "fn to_vec(b: &[u64]) -> Vec<u64>;",
    (
        pub fn to_vec(b: &[u64]) -> Vec<u64> {
            b.to_vec()
        }
    ),
    ( ),
    (
        assert(api.toVec([]).equals([]));
        assert(api.toVec([0, 1, 2, 3, 4]).equals([0, 1, 2, 3, 4]));
    ),
    (
        assert.deepEqual(api.to_vec([]), []);
        assert.deepEqual(api.to_vec([0n, 1n, 2n, 3n, 4n]), [0n, 1n, 2n, 3n, 4n]);
    ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        to_vec(b: Array<BigInt>): Array<BigInt>;

        drop(): void;
    })
}

compile_pass! {
    args_i64_ret_opt_i64,
    "fn non_zero(num: i64) -> Option<i64>;",
    (
        pub fn non_zero(num: i64) -> Option<i64> {
            if num > 0 {
                Some(num)
            } else {
                None
            }
        }
    ),
    ( ),
    (
        assert(api.nonZero(0) == null);
        assert(api.nonZero(42)! == 42);
    ),
    (
        assert.equal(api.non_zero(0n), null);
        assert.equal(api.non_zero(42n), 42);
    ),
    ( )
}

compile_pass! {
    args_i64_ret_res_i64,
    "fn non_zero(num: i64) -> Result<i64>;",
    (
        pub fn non_zero(num: i64) -> Result<i64, &'static str> {
            if num > 0 {
                Ok(num)
            } else {
                Err("is zero")
            }
        }
    ),
    ( ),
    (
        assert(api.nonZero(42) == 42);

        var err = false;
        try {
            api.nonZero(0);
        } catch(e) {
            err = true;
            assert(e == "is zero");
        }
        assert(err);
    ),
    (
        assert.equal(api.non_zero(42n), 42);

        let err = false;
        try {
            api.non_zero(0n);
        } catch(e) {
            err = true;
            assert.equal(e, "is zero");
        }
        assert.equal(err, true);
    ),
    ( )
}

compile_pass! {
    num_min_max,
    "
       fn u64_min() -> u64;
       fn u64_max() -> u64;
       fn i64_min() -> i64;
       fn i64_max() -> i64;
       fn u32_min() -> u32;
       fn u32_max() -> u32;
       fn i32_min() -> i32;
       fn i32_max() -> i32;
       fn i32_identity(v: i32) -> i32;
       fn i64_identity(v: i64) -> i64;
       fn u32_identity(v: u32) -> u32;
       fn u64_identity(v: u64) -> u64;
    ",
    (
        pub fn u64_min() -> u64 {
            u64::MIN
        }
        pub fn u64_max() -> u64 {
            u64::MAX
        }
        pub fn i64_min() -> i64 {
            i64::MIN
        }
        pub fn i64_max() -> i64 {
            i64::MAX
        }

        pub fn u32_min() -> u32 {
            u32::MIN
        }
        pub fn u32_max() -> u32 {
            u32::MAX
        }
        pub fn i32_min() -> i32 {
            i32::MIN
        }
        pub fn i32_max() -> i32 {
            i32::MAX
        }

        pub fn i32_identity(v: i32) -> i32 { v }
        pub fn i64_identity(v: i64) -> i64 { v }
        pub fn u32_identity(v: u32) -> u32 { v }
        pub fn u64_identity(v: u64) -> u64 { v }
    ),
    (),
    (
        var x = api.u64Min();
        assert(x == 0);
        assert(api.u64Identity(x) == x);

        x = api.u64Max();
        assert(x == 0xffffffffffffffff);
        assert(api.u64Identity(x) == x);

        x = api.i64Min();
        assert(x == -9223372036854775808);
        assert(api.i64Identity(x) == x);

        x = api.i64Max();
        assert(x == 9223372036854775807);
        assert(api.i64Identity(x) == x);

        x = api.i32Min();
        assert(x == -2147483648);
        assert(api.i32Identity(x) == x);

        x = api.i32Max();
        assert(x == 2147483647);
        assert(api.i32Identity(x) == x);

        x = api.u32Min();
        assert(x == 0);
        assert(api.u32Identity(x) == x);

        x = api.u32Max();
        assert(x == 0xffffffff);
        assert(api.u32Identity(x) == x);
    ),
    (
        let x = api.u64_min();
        assert.equal(x, 0n);
        assert.equal(api.u64_identity(x), x);

        x = api.u64_max();
        assert.equal(x, 0xffff_ffff_ffff_ffffn);
        assert.equal(api.u64_identity(x), x);

        x = api.i64_min();
        assert.equal(x, -9223372036854775808n);
        assert.equal(api.i64_identity(x), x);

        x = api.i64_max();
        assert.equal(x, 9223372036854775807n);
        assert.equal(api.i64_identity(x), x);

        x = api.i32_min();
        assert.equal(x, -2147483648);
        assert.equal(api.i32_identity(x), x);

        x = api.i32_max();
        assert.equal(x, 2147483647);
        assert.equal(api.i32_identity(x), x);

        x = api.u32_min();
        assert.equal(x, 0n);
        assert.equal(api.u32_identity(x), x);

        x = api.u32_max();
        assert.equal(x, 0xffff_ffff);
        assert.equal(api.u32_identity(x), x);
    ),
    ()
}
