// Guard for the leading-spread clone: every line must match tsx.
const log: string[] = [];
const show = (label: string, o: any) =>
  console.log(label, JSON.stringify(o), Object.keys(o).join(","), Object.getPrototypeOf(o) === Object.prototype);

const base = { phase: 1, charges: 12, t: 0 };
show("basic", { ...base, charges: 5 });
show("override-first", { charges: 9, ...base });
show("two-spreads", { ...base, ...{ t: 7, extra: "e" } });
const inner = { o: { deep: { k: 1 } } };
show("prop-get", { ...inner.o, z: 1 });
const src2 = { v: { id: 1 }, w: [1, 2] };
const c2 = { ...src2 };
console.log("identity", c2.v === src2.v, c2.w === src2.w);
c2.v = { id: 2 };
(c2 as any).added = true;
show("independent-src", src2);
show("independent-clone", c2);

const withGetter = { a: 1, get g() { log.push("getter"); return 5; } };
const cg = { ...withGetter };
console.log("getter", JSON.stringify(cg), log.join(","), JSON.stringify(Object.getOwnPropertyDescriptor(cg, "g")));

const s = Symbol("s");
const withSym: any = { a: 1 };
withSym[s] = 2;
const cs = { ...withSym };
console.log("symbol", cs[s], Object.getOwnPropertySymbols(cs).length, JSON.stringify(cs));

const hidden = { a: 1, b: 2 };
Object.defineProperty(hidden, "h", { value: 3, enumerable: false });
show("non-enumerable", { ...hidden });

Object.defineProperty(Object.prototype, "zz", {
  set(_v) { log.push("proto-setter"); },
  get() { return "proto"; },
  configurable: true,
});
const withZz = { zz: 1, q: 2 };
const cz = { ...withZz };
console.log("proto-setter", cz.zz, Object.hasOwn(cz, "zz"), log.includes("proto-setter"));
delete (Object.prototype as any).zz;

class P { x = 1; y = 2; m() { return 1; } }
const cp: any = { ...new P() };
console.log("class-src", JSON.stringify(cp), cp instanceof P, typeof cp.m);

show("null-src", { ...(null as any) });
show("undef-src", { ...(undefined as any) });
const arr = [1, 2];
show("array-src", { ...arr });
const str = "ab";
show("string-src", { ...(str as any) });
const numKeys = { 1: "a", b: 2, 0: "z" };
show("numeric-keys", { ...numKeys });

const frozen = Object.freeze({ f: 1, g: 2 });
const cf: any = { ...frozen };
cf.f = 9;
console.log("frozen-src", JSON.stringify(cf), Object.isFrozen(cf), Object.isFrozen(frozen));

const proto = { inherited: 1 };
const child = Object.create(proto);
child.own = 2;
show("proto-src", { ...child });

const deleted: any = { a: 1, b: 2, c: 3 };
delete deleted.b;
show("deleted-key", { ...deleted });

const wide: any = {};
for (let i = 0; i < 40; i++) wide["k" + i] = i;
const cw = { ...wide };
console.log("wide", Object.keys(cw).length, cw.k0, cw.k39);

const small = { a: 1 };
show("one-key", { ...small, b: 2, c: 3, d: 4 });
const empty = {};
show("empty-src", { ...empty, x: 1 });

// Garbage pressure: clones keep their object values alive across collections.
let total = 0;
const keep: any[] = [];
for (let i = 0; i < 200000; i++) {
  const src = { id: i, payload: { n: i }, tag: "t" + (i % 7) };
  const c = { ...src, extra: i };
  if (i % 1000 === 0) keep.push(c);
  total += c.payload.n + c.extra;
}
let check = 0;
for (const k of keep) check += k.payload.n + k.id + k.tag.length;
console.log("gc", total, keep.length, check);
const internal = { __perry_wk_entries: 1, ok: 2 };
show("internal-key-name", { ...internal });

function capturedSpread() {
  const captured = { n: 7 };
  return () => ({ ...captured });
}
console.log("captured", JSON.stringify(capturedSpread()()));
const warmed: any = { a: 1, b: 2 };
for (let i = 0; i < 8; i++) ({ ...warmed });
delete warmed.b;
show("warm-delete", { ...warmed });
warmed.b = 3;
Object.defineProperty(warmed, "a", { enumerable: false });
show("warm-hide", { ...warmed });
warmed[s] = 4;
const warmedClone = { ...warmed };
console.log("warm-symbol", warmedClone[s], Object.getOwnPropertySymbols(warmedClone).length);

const order: string[] = [];
const ordered = { get a() { order.push("spread"); return 1; } };
function allocatingOverride() {
  order.push("value");
  const pressure: any[] = [];
  for (let i = 0; i < 100000; i++) pressure.push({ n: i });
  return pressure[99999].n;
}
const orderedClone = { ...ordered, a: allocatingOverride(), b: (order.push("next"), 3) };
console.log("override-order", order.join(","), JSON.stringify(orderedClone));
show("wide-overrides", { ...wide, k40: 40, k41: 41, k42: 42, k43: 43 });
