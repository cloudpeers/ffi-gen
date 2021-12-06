## build
```sh
cargo build --target wasm32-unknown-unknown && cp ../target/wasm32-unknown-unknown/debug/api.wasm js
```

### run in node
```sh
node js/index.mjs
```

### run in the browser
```sh
npx --yes http-server js
```
