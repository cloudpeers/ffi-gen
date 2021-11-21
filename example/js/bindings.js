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

class Api {
  async fetch(url, imports) {
    this.instance = await fetchAndInstantiate(url, imports);
  }

  hello_world() {
    this.instance.exports.__hello_world();
  }
}

module.exports = {
  Api: Api,
};
