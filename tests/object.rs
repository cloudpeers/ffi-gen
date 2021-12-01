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

        pub fn create(value: u32) -> CustomType {
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
            pub fn new_(value: u32) -> Self {
                Self { value }
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

compile_pass! {
    nodelay_future,
    "create fn(value: u32) -> Future<u32>;",
    (
        pub async fn create(value: u32) -> u32 {
            value
        }
    ),
    (
        let _fut = __create(42);
        let _f = __create_future_poll;
        let _d = __create_future_drop;
    ),
    (
        final fut = api.create(42);
        assert(await fut == 42);
    ),
    (
        const fut = api.create(42);
        assert.equal(await fut, 42);
    ),
    ( )
}

compile_pass! {
    delayed_future,
    "create fn() -> Future<u64>; wake fn();",
    (
        static mut WAKER: Option<Waker> = None;
        static mut WOKEN: bool = false;

        pub struct Delayed;

        impl Future for Delayed {
            type Output = u64;

            fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                unsafe {
                    if !WOKEN {
                        WAKER = Some(cx.waker().clone());
                        Poll::Pending
                    } else {
                        Poll::Ready(42)
                    }
                }
            }
        }

        pub fn create() -> Delayed {
            Delayed
        }

        pub fn wake() {
            unsafe {
                if let Some(waker) = WAKER.take() {
                    WOKEN = true;
                    waker.wake();
                }
            }
        }
    ),
    (
        let fut = __create();
        let _poll = __create_future_poll;
        __create_future_drop(0, fut);
    ),
    (
        final fut = api.create();
        api.wake();
        assert(await fut == 42);
    ),
    (
        const i = setInterval(() => {
            // do nothing but prevent node process from exiting
        }, 1000);

        const fut = api.create();
        api.wake();
        console.log(fut);
        const res = await fut;
        clearInterval(i);
        console.log(res);
        assert.equal(res, 42);
    ),
    ( )
}

compile_pass! {
    nodelay_stream,
    "create fn(values: &[u32]) -> Stream<u32>;",
    (
        struct TestStream(Vec<u32>);

        impl Stream for TestStream {
            type Item = u32;

            fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Option<Self::Item>> {
                Poll::Ready(self.0.pop())
            }
        }

        pub fn create(values: &[u32]) -> impl Stream<Item = u32> {
            TestStream(values.into_iter().rev().copied().collect())
        }
    ),
    (
        let values = [42, 99];
        let stream = __create(values.as_ptr() as _, values.len() as _);
        let _poll = __create_stream_poll;
        __create_stream_drop(0, stream);
    ),
    (
        final stream = api.create([42, 99]);
        assert(await stream == 42);
        assert(await stream == 99);
        assert(await stream == null);
    ),
    (
        const stream = api.create([42, 99]);
        assert.equal(await stream, 42);
        assert.equal(await stream, 99);
        assert.equal(await stream, null);
    ),
    ( )
}
