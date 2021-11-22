use ffi_gen::compile_pass;

compile_pass! {
    boxed,
    r#"make_box fn(value: u32) -> Box<CustomType>;
    do_something fn(boxed: &CustomType) -> u32;
    was_dropped fn() -> bool;
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

        pub fn make_box(value: u32) -> Box<CustomType> {
            Box::new(CustomType { value })
        }

        pub fn do_something(boxed: &CustomType) -> u32 {
            boxed.value
        }

        pub fn was_dropped() -> bool {
            WAS_DROPPED.load(Ordering::SeqCst)
        }
    ),
    (
        let boxed = __make_box(42);
        assert_eq!(do_something(&boxed), 42);
        drop(boxed);
        assert!(was_dropped());
    ),
    (
        final boxed = api.make_box(42);
        assert(api.do_something(boxed) == 42);
        while (!api.was_dropped()) {}
    ),
    (
        const boxed = api.make_box(42);
        assert.equal(api.do_something(boxed), 42);
        while (!api.was_dropped()) {}
    ),
}
