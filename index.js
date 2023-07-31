import init, { hello, wtconnect } from "./pkg/wasm_webtransport.js";
await init();
hello();

self.wasm = {
    hello: hello,
    wtconnect: wtconnect,
}
