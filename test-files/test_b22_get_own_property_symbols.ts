// B22: Object.getOwnPropertySymbols lists symbol keys of plain, class and null-prototype objects.
const a = Symbol("a");
const b = Symbol.for("b");
const plain: Record<symbol | string, number> = { x: 1, [a]: 2, [b]: 3 };
console.log(Object.getOwnPropertySymbols(plain).length);
console.log(Object.getOwnPropertySymbols(plain).map((s) => s.toString()).join(","));
console.log(Object.getOwnPropertySymbols(plain)[0] === a, Object.getOwnPropertySymbols(plain)[1] === b);
const late: Record<symbol, number> = {};
console.log(Object.getOwnPropertySymbols(late).length);
late[a] = 1;
late[b] = 2;
console.log(Object.getOwnPropertySymbols(late).length);
class K {
  x = 1;
  [a] = 9;
  [Symbol.iterator]() {
    return [][Symbol.iterator]();
  }
}
const k = new K();
console.log(Object.getOwnPropertySymbols(k).length, Object.getOwnPropertySymbols(K.prototype).length);
const bare = Object.create(null);
bare[a] = 1;
console.log(Object.getOwnPropertySymbols(bare).length);
console.log(Object.getOwnPropertySymbols({ x: 1 }).length);
console.log(Reflect.ownKeys(plain).length);
console.log(Object.keys(plain).join(","), JSON.stringify(plain));
// Well-known Symbol members of a class prototype (lowered to synthetic slots).
class Prim {
  [Symbol.toPrimitive](): number {
    return 1;
  }
  async *[Symbol.asyncIterator](): AsyncGenerator<number> {
    yield 1;
  }
  get [Symbol.toStringTag](): string {
    return "Prim";
  }
}
console.log(Object.getOwnPropertySymbols(Prim.prototype).map((s) => s.toString()).join(","));
class Probe {
  static [Symbol.hasInstance](v: unknown): boolean {
    return v === 1;
  }
}
console.log(Object.getOwnPropertySymbols(Probe).map((s) => s.toString()).join(","));
// A class that captures its scope has a per-evaluation prototype object.
function makeNested(seed: number) {
  class Vec {
    x = seed;
    *[Symbol.iterator](): Generator<number> {
      yield this.x;
    }
    [Symbol.toPrimitive](): number {
      return this.x;
    }
  }
  return Vec;
}
const Vec = makeNested(3);
console.log(Object.getOwnPropertySymbols(Vec.prototype).length, Object.getOwnPropertySymbols(new Vec()).length);
console.log(Object.getOwnPropertySymbols(Vec.prototype).map(String).join(","));
