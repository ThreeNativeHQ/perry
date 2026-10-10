// Guard for the cached negative `in` answer: every mutation that can make an
// absent key present must flip the answer at the same site.
const out: string[] = [];
function probe(o: object): boolean {
  return "zz" in o;
}
const a: any = { x: 1 };
const b: any = { x: 2 };
for (let i = 0; i < 5; i++) probe(a);
out.push("plain " + probe(a) + " " + probe(b));
(Object.prototype as any).zz = 1;
out.push("proto-add " + probe(a) + " " + probe(b));
delete (Object.prototype as any).zz;
out.push("proto-del " + probe(a));
Object.defineProperty(Object.prototype, "zz", { get() { return 7; }, configurable: true });
out.push("proto-getter " + probe(a));
delete (Object.prototype as any).zz;
out.push("getter-del " + probe(a));
for (let i = 0; i < 5; i++) probe(a);
Object.setPrototypeOf(b, { zz: 3 });
out.push("setproto " + probe(a) + " " + probe(b));
a.zz = 0;
out.push("own-add " + probe(a));
delete a.zz;
out.push("own-del " + probe(a));
a.zz = 5;
out.push("own-readd " + probe(a));
const n: any = Object.create(null);
out.push("null-proto " + probe(n));
n.zz = 1;
out.push("null-own " + probe(n));
const f = Object.freeze({ y: 1 });
out.push("frozen " + probe(f) + " " + probe(f));
const arr: any = [1, 2];
out.push("array " + probe(arr));
arr.zz = 1;
out.push("array-own " + probe(arr));
class C { q = 1; }
const c = new C();
out.push("class " + probe(c));
(C.prototype as any).zz = 1;
out.push("class-proto " + probe(c));
const m: any = {};
for (let i = 0; i < 40; i++) m["k" + i] = i;
out.push("many " + probe(m));
m.zz = 1;
out.push("many-own " + probe(m));
console.log(out.join("\n"));
