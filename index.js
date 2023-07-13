import init, { hello } from "./pkg/wasm_webtransport.js";
await init();
hello();

window.wasm = {
    hello: hello
}
