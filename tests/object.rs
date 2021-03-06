use ffi_gen::compile_pass;

compile_pass! {
    object,
    r#"
    fn create(value: u32) -> CustomType;
    fn was_dropped() -> bool;
    object CustomType {
        static fn create(value: u32) -> CustomType;
        fn do_something() -> u32;
    }
    "#,
    (
        use std::sync::atomic::{AtomicBool, Ordering};

        static WAS_DROPPED: AtomicBool = AtomicBool::new(false);

        pub fn create(value: u32) -> CustomType {
            CustomType::create(value)
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
            pub fn create(value: u32) -> Self {
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
        let boxed = __CustomType_create(42);
        assert_eq!(__CustomType_do_something(boxed), 42);
        drop_box_CustomType(0 as _, boxed);
        assert!(was_dropped());

        let boxed = __create(42);
        assert_eq!(__CustomType_do_something(boxed), 42);
        drop_box_CustomType(0 as _, boxed);
        assert!(was_dropped());
    ),
    (
        final boxed = CustomType.create(api, 42);
        assert(boxed.doSomething() == 42);
        boxed.drop();
        assert(api.wasDropped());

        final obj = api.create(42);
        assert(obj.doSomething() == 42);
        obj.drop();
        assert(api.wasDropped());
    ),
    (
        const boxed = CustomType.create(api, 42);
        assert.equal(boxed.doSomething(), 42);
        boxed.drop();
        assert.equal(api.wasDropped(), true);

        const obj = api.create(42);
        assert.equal(obj.doSomething(), 42);
        obj.drop();
        assert.equal(api.wasDropped(), true);
    ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        create(value: number): CustomType;

        wasDropped(): boolean;
    }

    export class CustomType {
        static create(api: Api, value: number): CustomType;

        doSomething(): number;

        drop(): void;
    })

}

compile_pass! {
    iterator,
    r#"fn vec_str() -> Iterator<string>;
    //fn vec_vec_str() -> Iterator<Iterator<string>>;
    "#,
    (
        pub fn vec_str() -> Vec<String> {
            vec!["hello".into(), "world".into()]
        }

        /*pub fn vec_vec_str() -> Vec<Vec<String>> {
            vec![vec!["hello".into()], vec!["world".into()]]
        }*/
    ),
    (
        let iter = __vec_str();
        assert_eq!(__vec_str_iter_next(iter).ret0, 1);
        assert_eq!(__vec_str_iter_next(iter).ret0, 1);
        assert_eq!(__vec_str_iter_next(iter).ret0, 0);
        __vec_str_iter_drop(0, iter);
    ),
    (
        final List<String> res = [];
        for (final s in api.vecStr()) {
            res.add(s);
        }
        assert(res.length == 2);
        assert(res[0] == "hello");
        assert(res[1] == "world");

        /*final res = api.vecVecStr(); //[["hello"], ["world"]]);
        assert(res.length == 2);
        assert(res[0].length == 1);
        assert(res[0][0] == "hello");
        assert(res[1].length == 1);
        assert(res[1][0] == "world");*/
    ),
    (
        const res = [];
        let iter = api.vecStr();
        for (const el of iter) {
            res.push(el);
        }
        assert(res.length == 2);
        assert(res[0] == "hello");
        assert(res[1] == "world");
    ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        vecStr(): Iterable<string>;
    })
}

compile_pass! {
    nodelay_future,
    "fn create(value: u32) -> Future<u32>;",
    (
        pub async fn create(value: u32) -> u32 {
            value
        }
    ),
    (
        let _fut = __create(42);
        let _f = __create_future_poll;
        __create_future_drop(0, _fut);
    ),
    (
        final fut = api.create(42);
        assert(await fut == 42);
    ),
    (
        const fut = api.create(42);
        assert.equal(await fut, 42);
    ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        create(value: number): Promise<number>;
    })
}

compile_pass! {
    delayed_future,
    "fn create() -> Future<u64>; fn wake();",
    (
        use core::future::Future;
        use core::pin::Pin;
        use core::task::{Context, Poll, Waker};

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
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        create(): Promise<BigInt>;

        wake(): void;
    })
}

compile_pass! {
    nodelay_stream,
    "fn create(values: &[u32]) -> Stream<u32>;",
    (
        use crate::api::Stream;
        use core::pin::Pin;
        use core::task::{Context, Poll};

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

        extern "C" fn callback(port: i64, _obj: &i32) {
            assert!(port == 0 || port == 1);
        }

        let poll = __create_stream_poll(stream, callback as *const core::ffi::c_void as _, 0, 1);
        assert_eq!(poll.ret0, 1);
        assert_eq!(poll.ret1, 42);
        let poll = __create_stream_poll(stream, callback as *const core::ffi::c_void as _, 0, 1);
        assert_eq!(poll.ret0, 1);
        assert_eq!(poll.ret1, 99);
        let poll = __create_stream_poll(stream, callback as *const core::ffi::c_void as _, 0, 1);
        assert_eq!(poll.ret0, 0);

        __create_stream_drop(0, stream);
    ),
    (
        final stream = api.create([42, 99]);
        var counter = 0;
        await for (final value in stream) {
            assert(counter == 0 && value == 42 || counter == 1 && value == 99);
            counter += 1;
        }
        assert(counter == 2);
    ),
    (
        const i = setTimeout(() => {
            // do nothing but prevent node process from exiting
        }, 1000);

        const stream = api.create([42, 99]);
        let counter = 0;
        for await (const value of stream) {
            assert(counter == 0 && value == 42 || counter == 1 && value == 99);
            counter += 1;
        }
        assert(counter == 2);
        clearInterval(i);
    ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        create(values: Array<number>): ReadableStream<number>;
    })
}

compile_pass! {
    result_future,
    "fn create(value: u32) -> Future<Result<u32>>;",
    (
        pub async fn create(value: u32) -> Result<u32, &'static str> {
            if value == 0 {
                Err("is zero")
            } else {
                Ok(value)
            }
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

        var err = false;
        try {
            final fut = api.create(0);
            assert(await fut == 99);
        } catch(ex) {
            assert(ex == "is zero");
            err = true;
        }
        assert(err);
    ),
    (
        const fut = api.create(42);
        assert.equal(await fut, 42);

        let err = false;
        try {
            const fut = api.create(0);
            assert.equal(await fut, 99);
        } catch(ex) {
            assert.equal(ex, "is zero");
            err = true;
        }
        assert.equal(err, true);
    ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        create(value: number): Promise<number>;
    })
}

compile_pass! {
    future_iterator,
    "fn future_iterator() -> Future<Iterator<string>>;",
    (
        pub async fn future_iterator() -> Vec<String> {
            vec!["hello".to_string(), "world".to_string()]
        }
    ),
    ( ),
    (
        final iter = await api.futureIterator();
        final list = [];
        for (final item in iter) {
            list.add(item);
        }
        assert(list.length == 2);
        assert(list[0] == "hello");
        assert(list[1] == "world");
    ),
    (
        const iter = await api.futureIterator();
        const list = [];
        for (const item of iter) {
            list.push(item);
        }
        assert.equal(list.length, 2);
        assert.equal(list[0], "hello");
        assert.equal(list[1], "world");
    ),
    (
    export class Api {
        constructor();

        fetch(url, imports): Promise<void>;

        futureIterator(): Promise<Iterable<string>>;
    })
}
