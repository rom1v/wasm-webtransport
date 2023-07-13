import init, { hello, wtconnect } from "./pkg/wasm_webtransport.js";
await init();
hello();

window.wasm = {
    hello: hello,
    wtconnect: wtconnect,
}
