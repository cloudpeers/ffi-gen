const path = require('path');
const { Api } = require('./bindings');

async function main() {
    let cnt = 0;
    const fns = {};

    const api = new Api();
    await api.fetch(path.join(__dirname, 'api.wasm'), {
        env: {
            __console_log: (ptr, len) => {
                const buf = new Uint8Array(api.instance.exports.memory.buffer, ptr, len);
                const decoder = new TextDecoder();
                console.log(decoder.decode(buf));
            },
            call_function0: (idx) => {
                fns[idx]();
                fns[idx] = undefined;
            },
            call_function1: (idx, arg) => {
                fns[idx](arg);
                fns[idx] = undefined;
            },
        },
    });
    api.hello_world();

    var idx = cnt++;
    fns[idx] = () => { console.log("callback"); };
    api.instance.exports.invoke_callback0(idx);

    var idx = cnt++;
    fns[idx] = (arg) => { console.log(arg); };
    api.instance.exports.invoke_callback1(idx, 42);
}

main();
