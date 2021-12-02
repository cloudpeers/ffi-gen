let fs;
function fetch_polyfill(file) {
  return new Promise((resolve, reject) => {
    (fs || (fs = eval("equire".replace(/^/, "r"))("fs"))).readFile(
      file,
      function (err, data) {
        return err
          ? reject(err)
          : resolve({
              arrayBuffer: () => Promise.resolve(data),
              ok: true,
            });
      }
    );
  });
}

const { ReadableStream } =
  (typeof window == "object" && { ReadableStream }) ||
  require("node:stream/web");

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
      const nextNotifier = () => poll(controller.enqueue, nextIdx, doneIdx);
      const doneNotifier = () => {
        notifierRegistry.unregisterNotifier(nextIdx);
        notifierRegistry.unregisterNotifier(doneIdx);
        controller.close();
        box.drop();
      };
      notifierRegistry.registerNotifier(nextIdx, nextNotifier);
      notifierRegistry.registerNotifier(doneIdx, doneNotifier);
      poll(controller.enqueue, nextIdx, doneIdx);
    },
  });
};

class Api {
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

  hello_world() {
    this.instance.exports.__hello_world();
    return;
  }
}

module.exports = {
  Api: Api,
};
