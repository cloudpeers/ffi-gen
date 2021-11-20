const path = require('path');
const { Api } = require('./bindings');

async function main() {
    const api = new Api();
    await api.fetch(path.join(__dirname, 'api.wasm'), {
        env: {
            __console_log: (ptr, len) => {
                const buf = new Uint8Array(api.instance.exports.memory.buffer, ptr, len);
                const decoder = new TextDecoder();
                console.log(decoder.decode(buf));
            },
        },
    });
    api.hello_world();
}

main();
