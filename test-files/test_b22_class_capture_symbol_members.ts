// B22: pruned capture prologues and symbol-keyed computed members keep JS semantics.
interface IVec3 { x: number; y: number; z: number }
function defineMath(start: number) {
  let counter = start;
  const bump = (): number => ++counter;
  class Vector3 implements IVec3 {
    declare x: number;
    declare y: number;
    declare z: number;
    constructor(x = 0, y = 0, z = 0) {
      this.x = x;
      this.y = y;
      this.z = z;
    }
    set(x: number, y: number, z?: number): this {
      this.x = x;
      this.y = y;
      this.z = z === undefined ? this.z : z;
      return this;
    }
    tick(): number {
      return bump() + this.x;
    }
    get count(): number {
      return counter;
    }
    set count(value: number) {
      counter = value;
    }
    clone(): Vector3 {
      return new Vector3(this.x, this.y, this.z);
    }
    *[Symbol.iterator](): Generator<number> {
      yield this.x;
      yield this.y;
      yield this.z;
    }
    [Symbol.for("b22.tag")](): string {
      return `v(${this.x},${this.y},${this.z})`;
    }
  }
  return { Vector3, read: () => counter };
}
const { Vector3, read } = defineMath(10);
const v = new Vector3(1, 2, 3);
v.set(4, 5);
console.log(v.x, v.y, v.z, [...v].join(","));
console.log(v.tick(), v.tick(), read(), v.count);
console.log(read(), v.tick(), v.clone().set(7, 8, 9).z, v.clone().x);
console.log((v as unknown as Record<symbol, () => string>)[Symbol.for("b22.tag")]());
let sum = 0;
for (let i = 0; i < 1000; i++) sum += v.set(i, i * 2, i * 3).x + v.z;
console.log(sum);
// A wrapper over the same prototype whose lanes are own accessors.
const lanes = [0, 0, 0];
const member = Object.create(Vector3.prototype) as InstanceType<typeof Vector3>;
Object.defineProperties(member, {
  x: { get: () => lanes[0], set: (n: number) => { lanes[0] = n * 10; }, enumerable: true },
  y: { get: () => lanes[1], set: (n: number) => { lanes[1] = n * 10; }, enumerable: true },
  z: { get: () => lanes[2], set: (n: number) => { lanes[2] = n * 10; }, enumerable: true },
});
member.set(1, 2, 3);
console.log(lanes.join(","), member.x, [...member].join(","));
member.set(5, 6);
console.log(lanes.join(","));
// A string-keyed computed accessor still routes field access through it.
const key = "w";
class Boxed {
  declare w: number;
  hits = 0;
  get [key](): number {
    this.hits++;
    return 42;
  }
  set [key](value: number) {
    this.hits += value;
  }
  *[Symbol.iterator](): Generator<number> {
    yield this.w;
  }
}
const boxed = new Boxed();
boxed.w = 5;
console.log(boxed.w, boxed.hits, [...boxed].join(","));
// A constructor that calls a capture-reading method sees the capture.
function withCall(k: number) {
  class C {
    declare y: number;
    constructor() {
      this.y = this.read();
    }
    read(): number {
      return k * 2;
    }
  }
  return C;
}
const C = withCall(21);
console.log(new C().y, Object.keys(new C()).filter((name) => !name.startsWith("__")).join(","));
