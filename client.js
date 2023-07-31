import init, { WasmCtx } from "./pkg/wasm_webtransport.js";
await init();

self.wasm = WasmCtx.new();
