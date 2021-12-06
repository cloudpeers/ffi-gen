import * as pkg from "./bindings.mjs";

const { Api } = pkg;

const p = async () => {
  if (typeof window != "object") {
    const path = await import("path");
    const __dirname = path.dirname(import.meta.url.replace("file:", ""));
    return path.join(__dirname, "api.wasm");
  } else {
    return "api.wasm";
  }
};
async function main() {
  const api = new Api();
  const pa = await p();
  await api.fetch(pa, {
    env: {
      __console_log: (ptr, len) => {
        const buf = new Uint8Array(
          api.instance.exports.memory.buffer,
          ptr,
          len
        );
        const decoder = new TextDecoder();
        console.log(decoder.decode(buf));
      },
    },
  });
  api.helloWorld();
}

main();
