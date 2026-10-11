// PRD-582: `this` inside arrow closures and async step bodies, field initializers in a class
// without a constructor, and an async function with no await.
class Base {
  base = 10;
  arrow(): number {
    const f = () => this.base + 1;
    return f();
  }
  async plain(): Promise<number> {
    return this.base;
  }
  async load(n: number): Promise<number> {
    await 0;
    return n + this.base;
  }
}
class Derived extends Base {
  extra = 5;
  sum(): number {
    return this.base + this.extra;
  }
}
const b = new Base();
console.log("arrow " + b.arrow());
console.log("sum " + new Derived().sum());
b.plain().then((v) => console.log("plain " + v));
b.load(1).then((v) => console.log("load " + v));
