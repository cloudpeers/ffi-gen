use ffi_gen::compile_pass;

compile_pass! {
    object,
    r#"
    was_dropped fn() -> bool;
    object CustomType {
        static new_ fn(value: u32) -> Box<CustomType>;
        do_something fn() -> u32;
    }
    "#,
    (
        use std::sync::atomic::{AtomicBool, Ordering};

        static WAS_DROPPED: AtomicBool = AtomicBool::new(false);

        pub struct CustomType {
            value: u32,
        }

        impl Drop for CustomType {
            fn drop(&mut self) {
                WAS_DROPPED.store(true, Ordering::SeqCst);
            }
        }

        impl CustomType {
            pub fn new_(value: u32) -> Box<Self> {
                Box::new(Self { value })
            }

            pub fn do_something(&self) -> u32 {
                self.value
            }
        }

        pub fn was_dropped() -> bool {
            WAS_DROPPED.load(Ordering::SeqCst)
        }
    ),
    (
        let boxed = __CustomType_new_(42);
        assert_eq!(__CustomType_do_something(&boxed), 42);
        drop_box_CustomType(boxed);
        assert!(was_dropped());
    ),
    (
        final boxed = CustomType.new_(api, 42);
        assert(boxed.do_something() == 42);
        boxed.drop();
        assert(api.was_dropped());
        //while (!api.was_dropped()) {}
    ),
    (
        const boxed = new CustomType(api, 42);
        assert.equal(boxed.do_something(), 42);
        boxed.drop();
        assert.equal(api.was_dropped(), true);
        //while (!api.was_dropped()) {}
    ),
}
