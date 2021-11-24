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

const fetchFn = (typeof fetch === "function" && fetch) || fetch_polyfill;

function fetchAndInstantiate(url, imports) {
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
    const ret = this.instance.exports.__hello_world();
  }
}

module.exports = {
  Api: Api,
};
