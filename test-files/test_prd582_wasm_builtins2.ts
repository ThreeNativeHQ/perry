// PRD-582: the rarer builtins in Midway's graph that the Wasm emitter lowered to `undefined`.
const proto = { kind: "proto" };
const made = Object.create(proto) as { kind: string };
const reproto = Object.setPrototypeOf({}, proto) as { kind: string };
const multi = Object.defineProperties({} as Record<string, number>, { a: { value: 1, enumerable: true }, b: { value: 2 } });
console.log(`object ${made.kind} ${reproto.kind} ${multi.a + multi.b} ${Object.keys(multi).join("|")} ${Object.getOwnPropertyNames(multi).join("|")}`);
console.log(`is ${Object.is(NaN, NaN)} ${Object.is(0, -0)} at ${[1, 2, 3].at(-1)}`);

const bag: Record<string, number> = {};
Reflect.set(bag, "k", 5);
console.log(`reflect ${bag.k} uri ${encodeURIComponent("a b&c")} ${decodeURIComponent("a%20b")}`);

const bytes = new TextEncoder().encode("hé");
console.log(`text ${bytes.length} ${new TextDecoder().decode(bytes)}`);

const re = new RegExp("(\\d+)-(\\d+)", "g");
const m = re.exec("10-20");
console.log(`regex ${m?.[1]} ${m?.[2]}`);
const parsed = JSON.parse('{"n": 3}') as { n: number };
interface ITyped {
  n: number;
}
const typed: ITyped = JSON.parse('{"n": 4}');
const list: number[] = [7, 8, 9];
console.log(`json ${parsed.n} ${typed.n} at ${list.at(-2)}`);

class Point {
  constructor(readonly x: number, readonly y: number) {}
}
class Point3 extends Point {
  z: number;
  constructor(x: number, y: number, z: number) {
    const xy: [number, number] = [x, y];
    super(...xy);
    this.z = z;
  }
}
const args: [number, number] = [4, 5];
const viaSpread = new Point(...args);
const p3 = new Point3(1, 2, 3);
console.log(`spread-new ${viaSpread.x + viaSpread.y} super ${p3.x + p3.y + p3.z}`);

const handler = { get: (t: Record<string, number>, k: string) => (k in t ? t[k] : -1) };
const prox = new Proxy({ a: 1 } as Record<string, number>, handler);
console.log(`proxy ${prox.a} ${prox.missing}`);
