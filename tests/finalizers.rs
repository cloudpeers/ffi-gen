use ffi_gen::compile_pass;

compile_pass! {
    finalizers,
    r#"fn make_box() -> CustomType;
    fn was_dropped() -> bool;
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

        pub fn make_box() -> CustomType {
            CustomType
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
        final f = () {
            final boxed = api.makeBox();
        };
        f();
        //boxed.drop();
        //assert(api.wasDropped());
        final largeList = [];
        while (!api.wasDropped()) {
            //sleep(Duration(milliseconds: 10));
            largeList.add(99);
        }
    ),
    (
        const f = () => {
            let boxed = api.makeBox();
        };
        f();
        const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
        //boxed.drop();
        //assert.equal(api.was_dropped(), true);
        while (!api.wasDropped()) {
            await delay(1000);
            global.gc();
        }
    ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        makeBox(): CustomType;

        wasDropped(): boolean;
    }

    export class CustomType {

        drop(): void;
    }
    )

}
