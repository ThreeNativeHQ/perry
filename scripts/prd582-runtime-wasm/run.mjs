// Usage: node run.mjs <perry_runtime.wasm> <rt_math.wasm>
// Instantiates perry-runtime (wasm32-wasip1) and the k_math kernel module that imports its
// exports directly: objects and arrays live in the runtime's linear memory and no value op
// crosses into JS. Exits 1 when the kernel result differs from Node running k_math.ts.
import fs from "node:fs";
import { WASI } from "node:wasi";
const [rtPath, kernelPath] = process.argv.slice(2);
const wasi = new WASI({ version: "preview1", returnOnExit: true });
const rtMod = await WebAssembly.compile(fs.readFileSync(rtPath));
const imports = { wasi_snapshot_preview1: wasi.wasiImport };
const unbound = [];
for (const { module, name, kind } of WebAssembly.Module.imports(rtMod)) {
  if (module === "wasi_snapshot_preview1" || kind !== "function") continue;
  imports[module] ??= {};
  imports[module][name] = () => { throw new Error(`PERRY_WASM_UNBOUND: ${module}.${name}`); };
  unbound.push(`${module}.${name}`);
}
const rt = await WebAssembly.instantiate(rtMod, imports);
wasi.initialize(rt);
const { instance } = await WebAssembly.instantiate(fs.readFileSync(kernelPath), { rt: rt.exports });
const ms = [];
let acc = 0;
for (let r = 0; r < 5; r++) {
  const t0 = performance.now();
  acc = instance.exports.run(200000);
  ms.push(+(performance.now() - t0).toFixed(1));
}
console.log(`runtime exports ${WebAssembly.Module.exports(rtMod).length}; unbound native imports ${unbound.length}: ${unbound.join(" ")}`);
console.log(`math acc=${Math.round(acc)} ms=${JSON.stringify(ms)}`);
if (Math.round(acc) !== 22199900) { console.error("FAIL: kernel result differs from Node"); process.exit(1); }
