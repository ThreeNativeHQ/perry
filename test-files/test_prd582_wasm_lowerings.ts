// PRD-582: HIR forms the Wasm emitter used to lower to `undefined`.
const probes: unknown[] = [1, 1.5, -0, NaN, Infinity, 2 ** 53, "3", null, undefined, true];
console.log(probes.map((v) => [Number.isFinite(v), Number.isInteger(v), Number.isSafeInteger(v)].join("/")).join(" "));

class Counter {
  #count = 0;
  static #made = 0;
  #step(): number {
    return 2;
  }
  get count(): number {
    return this.#count;
  }
  bump(): this {
    this.#count += this.#step();
    Counter.#made++;
    return this;
  }
  static made(): number {
    return Counter.#made;
  }
}
const c = new Counter().bump().bump();
console.log(`count ${c.count} made ${Counter.made()}`);

function* inner() {
  yield 1;
  yield 2;
}
function* outer() {
  yield 0;
  yield* inner();
  yield* [3, 4];
}
const seen: number[] = [];
for (const v of outer()) seen.push(v);
console.log("gen " + seen.join(","));
const nums = [5, 6];
let total = 0;
for (const n of nums) total += n;
console.log("forof " + total);
