// B15: keep ordinary closure calls, URLSearchParams subclasses (#6710), and
// Date cells (#5964) working across first use of rare dispatch features.
function invoke(receiver: any, name: string): any {
  return receiver[name]();
}
function check(actual: any, expected: any): void {
  if (actual !== expected) throw new Error("dynamic dispatch mismatch");
}
const own: any = { endFrame: () => 17 };
check(invoke(own, "endFrame"), 17);
class Budget {
  endFrame(): number { return 23; }
}
check(invoke(new Budget(), "endFrame"), 23);
const proto = { marker: 31, endFrame() { return this.marker; } };
check(invoke(Object.create(proto), "endFrame"), 31);
class Params extends URLSearchParams {}
const params: any = new Params("a=1&b=2");
check(invoke(params, "toString"), "a=1&b=2");
check(invoke(new Date(0), "getUTCFullYear"), 1970);
check(invoke(own, "endFrame"), 17);
console.log("B15 dispatch: 6 checks passed");
