// Usage: node scripts/prd582-run-wasm.cjs <perry --target wasm output.html>
// Runs a Perry Wasm HTML page under Node's WebAssembly with the same minimal
// DOM stubs as tests/wasm/run_wasm_tests.sh. Exit 1 with WASM_ERROR on a boot
// or runtime failure.
const fs = require("fs");
globalThis.window = globalThis;
const stub = { id: "", appendChild: () => {}, getElementById: () => null, style: {}, textContent: "" };
const doc = { createElement: () => Object.assign({}, stub), getElementById: () => null, title: "" };
doc.head = stub;
doc.body = stub;
globalThis.document = doc;
globalThis.requestAnimationFrame = (cb) => setTimeout(() => cb(performance.now()), 0);
globalThis.cancelAnimationFrame = (id) => clearTimeout(id);
globalThis.atob = (x) => Buffer.from(x, "base64").toString("binary");
const html = fs.readFileSync(process.argv[2], "utf8");
const scripts = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].map((m) => m[1]);
const source = scripts.join("\n").replace(/^(bootPerryWasm\()/m, "await $1");
eval(`(async()=>{${source}})()`).catch((e) => {
  console.error("WASM_ERROR:", e.message);
  process.exit(1);
});
