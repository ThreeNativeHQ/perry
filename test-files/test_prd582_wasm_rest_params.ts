// PRD-582: `...rest` parameters collect every remaining argument on each call path.
function sum(...xs: number[]): number {
  let total = 0;
  for (const x of xs) total += x;
  return total;
}
function tagged(tag: string, ...xs: number[]): string {
  return `${tag}:${xs.length}:${xs.join("+")}`;
}
const arrow = (...parts: string[]): string => parts.join("/");
const asValue: (...xs: number[]) => number = sum;

class Point {
  constructor(readonly x: number, readonly y: number) {}
}
class Vec extends Point {
  readonly rest: number[];
  constructor(...xyz: number[]) {
    super(xyz[0], xyz[1]);
    this.rest = xyz.slice(2);
  }
  scaled(k: number, ...extra: number[]): number {
    return (this.x + this.y) * k + extra.length;
  }
  static of(...xs: number[]): Vec {
    return new Vec(...xs);
  }
}
class Vec3 extends Vec {
  constructor(x: number, y: number, z: number) {
    super(x, y, z);
  }
}

const v = new Vec(1, 2, 3, 4);
const v3 = new Vec3(5, 6, 7);
console.log(`direct ${sum()} ${sum(1)} ${sum(1, 2, 3)} ${tagged("t")} ${tagged("t", 4, 5)}`);
console.log(`closure ${arrow("a", "b", "c")} ${asValue(4, 5, 6)} ${[[1, 2], [3]].map((p) => sum(...p)).join(",")}`);
console.log(`class ${v.x} ${v.y} ${v.rest.join(",")} ${v3.rest.join(",")} ${v.scaled(2, 9, 9)} ${Vec.of(7, 8).x}`);
