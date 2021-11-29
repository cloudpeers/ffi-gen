use ffi_gen::compile_pass;

compile_pass! {
    finalizers,
    r#"make_box fn() -> CustomType;
    was_dropped fn() -> bool;
    object CustomType {}
    "#,
    (
        use std::sync::atomic::{AtomicBool, Ordering};

        static WAS_DROPPED: AtomicBool = AtomicBool::new(false);

        pub struct CustomType;

        impl Drop for CustomType {
            fn drop(&mut self) {
                WAS_DROPPED.store(true, Ordering::SeqCst);
            }
        }

        pub fn make_box() -> Box<CustomType> {
            Box::new(CustomType)
        }

        pub fn was_dropped() -> bool {
            WAS_DROPPED.load(Ordering::SeqCst)
        }
    ),
    (
        let boxed = __make_box();
        drop_box_CustomType(0, boxed);
        assert!(was_dropped());
    ),
    (
        final boxed = api.make_box();
        //boxed.drop();
        //assert(api.was_dropped());
        final largeList = [];
        while (!api.was_dropped()) {
            //sleep(Duration(milliseconds: 10));
            largeList.add(99);
        }
    ),
    (
        const f = () => {
            let boxed = api.make_box();
        };
        f();
        const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
        //boxed.drop();
        //assert.equal(api.was_dropped(), true);
        while (!api.was_dropped()) {
            await delay(1000);
            global.gc();
        }
    ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        make_box(): CustomType;

        was_dropped(): boolean;

        drop(): void;
    }

    export class CustomType {

        drop(): void;
    }
    )

}
