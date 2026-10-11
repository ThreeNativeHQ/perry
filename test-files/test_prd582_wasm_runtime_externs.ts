// PRD-582: runtime helpers the Wasm emitter left unresolved (object spread, Map/Set iteration,
// WeakMap/WeakSet, computed keys, accessors, globals, iterator protocol, some/every, hasOwn).
const base = { a: 1, b: 2 };
const spread = { ...base, b: 3, c: 4 };
console.log("spread " + JSON.stringify(spread));

const m = new Map<string, number>([["x", 1], ["y", 2], ["z", 3]]);
const pairs: string[] = [];
for (const [k, v] of m) pairs.push(k + v);
for (const k of m.keys()) if (k === "y") m.delete(k);
const s = new Set<number>([4, 5, 6]);
let sum = 0;
for (const v of s) {
  sum += v;
  if (v === 4) s.delete(5);
}
console.log(`map ${pairs.join(",")} ${m.size} set ${sum} ${s.size}`);

const wm = new WeakMap<object, number>();
const ws = new WeakSet<object>();
const key = {};
wm.set(key, 7);
ws.add(key);
console.log(`weak ${wm.get(key)} ${wm.has(key)} ${ws.has(key)} ${wm.delete(key)} ${wm.has(key)}`);

const name = "dyn";
const literal = {
  [name]: 1,
  [`${name}Fn`]: () => 2,
  get twice(): number {
    return this.base * 2;
  },
  base: 21,
};
console.log(`literal ${literal.dyn} ${literal.dynFn()} ${literal.twice} ${Object.hasOwn(literal, "base")}`);

const nums = [1, 2, 3];
console.log(`some ${nums.some((n) => n > 2)} every ${nums.every((n) => n > 0)} ${nums.every((n) => n > 1)}`);

class Range implements Iterable<number> {
  constructor(private readonly end: number) {}
  [Symbol.iterator](): Iterator<number> {
    let i = 0;
    const end = this.end;
    return { next: () => (i < end ? { value: i++, done: false } : { value: undefined, done: true }) };
  }
}
const seen: number[] = [];
for (const v of new Range(3)) seen.push(v);
console.log("range " + seen.join(","));
console.log("global " + (typeof globalThis.setTimeout) + " " + (typeof (globalThis as Record<string, unknown>).noSuchGlobal));
