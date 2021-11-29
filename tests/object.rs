use ffi_gen::compile_pass;

compile_pass! {
    object,
    r#"
    create fn(value: u32) -> CustomType;
    was_dropped fn() -> bool;
    object CustomType {
        static new_ fn(value: u32) -> CustomType;
        do_something fn() -> u32;
    }
    "#,
    (
        use std::sync::atomic::{AtomicBool, Ordering};

        static WAS_DROPPED: AtomicBool = AtomicBool::new(false);

        pub fn create(value: u32) -> Box<CustomType> {
            CustomType::new_(value)
        }

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
        assert_eq!(__CustomType_do_something(boxed), 42);
        drop_box_CustomType(0 as _, boxed);
        assert!(was_dropped());

        let boxed = __create(42);
        assert_eq!(__CustomType_do_something(boxed), 42);
        drop_box_CustomType(0 as _, boxed);
        assert!(was_dropped());
    ),
    (
        final boxed = CustomType.new_(api, 42);
        assert(boxed.do_something() == 42);
        boxed.drop();
        assert(api.was_dropped());

        final obj = api.create(42);
        assert(obj.do_something() == 42);
        obj.drop();
        assert(api.was_dropped());
    ),
    (
        const boxed = CustomType.new_(api, 42);
        assert.equal(boxed.do_something(), 42);
        boxed.drop();
        assert.equal(api.was_dropped(), true);

        const obj = api.create(42);
        assert.equal(obj.do_something(), 42);
        obj.drop();
        assert.equal(api.was_dropped(), true);
    ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        create(value: number): CustomType;

        was_dropped(): boolean;

        drop(): void;
    }

    export class CustomType {
        static new_(api: Api, value: number): CustomType;

        do_something(): number;

        drop(): void;
    })

}
