hello_world fn();

create_future fn() -> Box<FfiFuture>;
poll_future fn(&FfiFuture) -> Option<u64>;
