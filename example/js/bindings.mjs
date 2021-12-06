let fs;
const fetch_polyfill = async (file) => {
  const readFile = await import("fs").then(({ readFile }) => readFile);
  return new Promise((resolve, reject) => {
    readFile(file, function (err, data) {
      return err
        ? reject(err)
        : resolve({
            arrayBuffer: () => Promise.resolve(data),
            ok: true,
          });
    });
  });
};

let ReadableStream;
if (typeof window == "object") {
  ReadableStream = window.ReadableStream;
} else {
  import("node:stream/web").then((pkg) => {
    ReadableStream = pkg.ReadableStream;
  });
}

const fetchFn = (typeof fetch === "function" && fetch) || fetch_polyfill;

function fetchAndInstantiate(url, imports) {
  const env = imports.env || {};
  env.__notifier_callback = (idx) => notifierRegistry.callbacks[idx]();
  imports.env = env;
  return fetchFn(url)
    .then((resp) => {
      if (!resp.ok) {
        throw new Error("Got a ${resp.status} fetching wasm @ ${url}");
      }

      const wasm = "application/wasm";
      const type = resp.headers && resp.headers.get("content-type");

      return WebAssembly.instantiateStreaming && type === wasm
        ? WebAssembly.instantiateStreaming(resp, imports)
        : resp
            .arrayBuffer()
            .then((buf) => WebAssembly.instantiate(buf, imports));
    })
    .then((result) => result.instance);
}

const dropRegistry = new FinalizationRegistry((drop) => drop());

class Box {
  constructor(ptr, destructor) {
    this.ptr = ptr;
    this.dropped = false;
    this.moved = false;
    dropRegistry.register(this, destructor);
    this.destructor = destructor;
  }

  borrow() {
    if (this.dropped) {
      throw new Error("use after free");
    }
    if (this.moved) {
      throw new Error("use after move");
    }
    return this.ptr;
  }

  move() {
    if (this.dropped) {
      throw new Error("use after free");
    }
    if (this.moved) {
      throw new Error("can't move value twice");
    }
    this.moved = true;
    dropRegistry.unregister(this);
    return this.ptr;
  }

  drop() {
    if (this.dropped) {
      throw new Error("double free");
    }
    if (this.moved) {
      throw new Error("can't drop moved value");
    }
    this.dropped = true;
    dropRegistry.unregister(this);
    this.destructor();
  }
}

class NotifierRegistry {
  constructor() {
    this.counter = 0;
    this.callbacks = {};
  }

  reserveSlot() {
    const idx = this.counter;
    this.counter += 1;
    return idx;
  }

  registerNotifier(idx, notifier) {
    this.callbacks[idx] = notifier;
  }

  unregisterNotifier(idx) {
    delete this.callbacks[idx];
  }
}

const notifierRegistry = new NotifierRegistry();

const nativeFuture = (box, nativePoll) => {
  const poll = (resolve, reject, idx) => {
    try {
      console.log(poll);
      const ret = nativePoll(box.borrow(), 0, BigInt(idx));
      console.log(ret);
      if (ret == null) {
        return;
      }
      resolve(ret);
    } catch (err) {
      console.log("error", err);
      reject(err);
    }
    notifierRegistry.unregisterNotifier(idx);
    box.drop();
  };
  return new Promise((resolve, reject) => {
    const idx = notifierRegistry.reserveSlot();
    const notifier = () => poll(resolve, reject, idx);
    notifierRegistry.registerNotifier(idx, notifier);
    poll(resolve, reject, idx);
  });
};

const nativeStream = (box, nativePoll) => {
  const poll = (next, nextIdx, doneIdx) => {
    const ret = nativePoll(box.borrow(), 0, BigInt(nextIdx), BigInt(doneIdx));
    if (ret != null) {
      next(ret);
    }
  };
  return new ReadableStream({
    start(controller) {
      const nextIdx = notifierRegistry.reserveSlot();
      const doneIdx = notifierRegistry.reserveSlot();
      const nextNotifier = () =>
        setImmediate(() =>
          poll((x) => controller.enqueue(x), nextIdx, doneIdx)
        );
      const doneNotifier = () => {
        notifierRegistry.unregisterNotifier(nextIdx);
        notifierRegistry.unregisterNotifier(doneIdx);
        controller.close();
        box.drop();
      };
      notifierRegistry.registerNotifier(nextIdx, nextNotifier);
      notifierRegistry.registerNotifier(doneIdx, doneNotifier);
      nextNotifier();
    },
  });
};

export class Api {
  async fetch(url, imports) {
    this.instance = await fetchAndInstantiate(url, imports);
  }

  allocate(size, align) {
    return this.instance.exports.allocate(size, align);
  }

  deallocate(ptr, size, align) {
    this.instance.exports.deallocate(ptr, size, align);
  }

  drop(symbol, ptr) {
    this.instance.exports[symbol](0, ptr);
  }

  helloWorld() {
    this.instance.exports.__hello_world();
    return;
  }
  asyncHelloWorld() {
    const tmp0 = this.instance.exports.__async_hello_world();
    const tmp2 = tmp0;
    const tmp2_0 = () => {
      this.drop("__async_hello_world_future_drop", tmp2);
    };
    const tmp2_1 = new Box(tmp2, tmp2_0);
    const tmp1 = nativeFuture(tmp2_1, (a, b, c) => {
      return this.asyncHelloWorldFuturePoll(a, b, c);
    });
    return tmp1;
  }
  asyncHelloWorldFuturePoll(boxed, postCobject, port) {
    const tmp0 = boxed;
    const tmp1 = tmp0;
    const tmp2 = postCobject;
    const tmp3 = tmp2;
    const tmp4 = port;
    const tmp5 = tmp4;
    const tmp6 = this.instance.exports.__async_hello_world_future_poll(
      tmp1,
      tmp3,
      tmp5
    );
    const tmp8 = tmp6[0];
    const tmp9 = tmp6[1];
    const tmp10 = tmp6[2];
    const tmp11 = tmp6[3];
    const tmp12 = tmp6[4];
    const tmp13 = tmp6[5];
    if (tmp8 === 0) {
      return null;
    }
    if (tmp9 === 0) {
      const tmp9_0 = new Uint8Array(
        this.instance.exports.memory.buffer,
        tmp10,
        tmp11
      );
      const tmp9_1 = new TextDecoder();
      const tmp9_2 = tmp9_1.decode(tmp9_0);
      if (tmp11 > 0) {
        this.deallocate(tmp10, tmp12, 1);
      }
      throw tmp9_2;
    }
    const tmp7 = tmp13;
    return tmp7;
  }
}

export default Api;
