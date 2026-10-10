function make() {
    class Point {
        declare x: number;
        constructor(x = 1) { this.x = x; }
        bump() { this.x += 1; }
        multiply(s: number) { this.x *= s; }
        read() { return this.x; }
    }
    return Point;
}
const Point = make();
const memory = [1];
const seen: object[] = [];
const accessors = Object.create(Point.prototype);
Object.defineProperty(accessors, "x", {
    configurable: true,
    get: function () { return memory[0]; },
    set: function (v: number) { seen.push(this); memory[0] = v; },
});
const value = Object.create(accessors);
value.multiply(6);
console.log(memory[0] === 6, value.read() === 6, Object.keys(value).length === 0);
value.x = 2;
console.log(memory[0] === 2, value.x === 2, seen[seen.length - 1] === value);
value.bump();
console.log(memory[0] === 3, value.read() === 3, seen[seen.length - 1] === value);
const deeper = Object.create(Object.create(accessors));
deeper.x = 8;
console.log(memory[0] === 8, deeper.read() === 8, Object.keys(deeper).length === 0);
const plain = new Point(2); plain.bump();
console.log(plain.x === 3, plain.read() === 3, Object.keys(plain).join(",") === "x");
class Sub extends Point {
    get x() { return memory[0]; }
    set x(v: number) { memory[0] = v; }
}
const sub = new Sub(1); sub.multiply(5);
console.log(memory[0] === 5, sub.read() === 5, Object.keys(sub).length === 0);
class StaticBase { constructor() { this.x = 1; this.y = 2; } bump() { this.x += 1; this.y += 1; } }
class StaticSub extends StaticBase {
    get x() { return memory[0]; }
    set x(v: number) { memory[0] = v; }
}
const staticSub = new StaticSub(); staticSub.bump();
console.log(memory[0] === 2, staticSub.x === 2, staticSub.y === 3);
const own = Object.create(Point.prototype);
Object.defineProperty(own, "x", { get: () => memory[0], set: (v: number) => { memory[0] = v; } });
own.x = 9; own.bump();
console.log(memory[0] === 10, own.read() === 10, own.x === 10);
const nearer = Object.create(accessors);
Object.defineProperty(nearer, "x", { value: 20, writable: true });
const child = Object.create(nearer); child.bump();
console.log(child.x === 21, child.read() === 21, memory[0] === 10);
const replacement = Object.create(Point.prototype);
Object.defineProperty(replacement, "x", { get: () => memory[0], set: (v: number) => { memory[0] = v; } });
const changed = new Point(1); delete changed.x; Object.setPrototypeOf(changed, replacement); changed.bump();
console.log(memory[0] === 11, changed.read() === 11, Object.keys(changed).length === 0);
