// Guard for shared integrity successors: frozen siblings must keep their own
// values, their frozen semantics, and stay distinct from unfrozen siblings.
"use strict";
const out: string[] = [];
const fs: any[] = [];
for (let i = 0; i < 6; i++) fs.push(Object.freeze({ a: i, b: 2 }));
const plain: any = { a: 100, b: 2 };
const sealed: any = Object.seal({ a: 200, b: 2 });
const nox: any = Object.preventExtensions({ a: 300, b: 2 });
function readA(o: any): number { return o.a; }
function writeA(o: any, v: number): string {
  try { o.a = v; return "ok:" + o.a; } catch (e) { return "throw:" + o.a; }
}
function addC(o: any): string {
  try { o.c = 1; return "ok:" + o.c; } catch (e) { return "throw:" + o.c; }
}
for (let r = 0; r < 3; r++) {
  let s = 0;
  for (const o of fs) s += readA(o) + o.b;
  out.push("sum " + s + " plain " + readA(plain) + " sealed " + readA(sealed) + " nox " + readA(nox));
}
out.push("w " + writeA(fs[0], 9) + " " + writeA(plain, 9) + " " + writeA(sealed, 9) + " " + writeA(nox, 9));
out.push("w2 " + writeA(fs[1], 8) + " " + writeA(plain, 8));
out.push("add " + addC(fs[0]) + " " + addC(plain) + " " + addC(sealed) + " " + addC(nox));
out.push("isFrozen " + Object.isFrozen(fs[2]) + " " + Object.isFrozen(plain) + " " + Object.isFrozen(sealed) + " " + Object.isFrozen(nox));
out.push("isSealed " + Object.isSealed(fs[2]) + " " + Object.isSealed(sealed) + " " + Object.isSealed(nox) + " " + Object.isSealed(plain));
out.push("ext " + Object.isExtensible(fs[3]) + " " + Object.isExtensible(nox) + " " + Object.isExtensible(plain));
out.push("del " + (() => { try { delete fs[4].a; return "ok"; } catch (e) { return "throw"; } })() + " " + fs[4].a);
out.push("desc " + JSON.stringify(Object.getOwnPropertyDescriptor(fs[5], "a")) + " " + JSON.stringify(Object.getOwnPropertyDescriptor(plain, "a")));
out.push("keys " + Object.keys(fs[5]).join(",") + " " + JSON.stringify(fs[5]));
const g = Object.freeze({ a: 1, b: 2, c: 3 });
const h = Object.freeze({ a: 4, b: 5, c: 6 });
out.push("three " + (g.a + g.c) + " " + (h.a + h.c) + " " + Object.isFrozen(h));
const late: any = { a: 7, b: 2 };
for (let i = 0; i < 3; i++) writeA(late, 7 + i);
Object.freeze(late);
out.push("late " + writeA(late, 99) + " " + addC(late));
console.log(out.join("\n"));
