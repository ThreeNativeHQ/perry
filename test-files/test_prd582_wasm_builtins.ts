// PRD-582: builtins Midway's graph uses that the Wasm emitter lowered to `undefined`.
const f32 = new Float32Array(3);
f32[1] = 1.5;
const u8 = new Uint8Array([1, 2, 300]);
console.log(`typed ${f32.length} ${f32[1]} ${u8[2]} ${u8.length}`);

const t0 = performance.now();
console.log("now " + (typeof t0) + " " + (performance.now() >= t0));

function add3(a: number, b: number, c: number): number {
  return a + b + c;
}
const nums = [1, 2, 3];
const holder = { add3 };
console.log(`spread ${add3(...nums)} ${holder.add3(...nums)} ${Math.max(...nums)}`);

const target: Record<string, unknown> = {};
Object.defineProperty(target, "fixed", { value: 7, enumerable: false });
Object.defineProperty(target, "computed", { get: () => 9, enumerable: true });
const frozen = Object.freeze({ a: 1 });
console.log(`define ${target.fixed} ${target.computed} ${Object.keys(target).join("|")} ${Object.isFrozen(frozen)}`);

const entries = Object.fromEntries([["x", 1], ["y", 2]]) as Record<string, number>;
const forIn: string[] = [];
for (const k in entries) forIn.push(k);
const desc = Object.getOwnPropertyDescriptor(entries, "x");
console.log(`entries ${entries.x + entries.y} ${forIn.join(",")} ${desc?.value} ${typeof Object.getPrototypeOf(entries)}`);

const bag: Record<string, number> = { k: 4 };
console.log(`reflect ${Reflect.get(bag, "k")} ${Reflect.deleteProperty(bag, "k")} ${"k" in bag}`);
console.log(`bool ${Boolean(0)} ${Boolean("x")} log ${Math.log(Math.E)}`);

class Base {
  hello(): string {
    return "base";
  }
}
const parents = [Base];
class Child extends parents[0] {
  hi(): string {
    return "child+" + this.hello();
  }
}
console.log(new Child().hi());
