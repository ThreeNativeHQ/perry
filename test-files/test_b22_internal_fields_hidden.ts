// B22: accessor keys and internal capture fields never show up as own enumerable data keys.
function make() {
  let hidden = 1;
  const stamp = (): number => hidden++;
  class Thing {
    a = 1;
    b = 2;
    get acc(): number {
      return hidden;
    }
    set acc(v: number) {
      hidden = v;
    }
    method(): number {
      return stamp();
    }
  }
  const lit = {
    p: 1,
    get acc2(): number {
      return hidden;
    },
    set acc2(v: number) {
      hidden = v;
    },
    q: 2,
    f: () => hidden,
  };
  return { Thing, lit };
}
const { Thing, lit } = make();
const t = new Thing();
t.method();
for (const o of [t, lit] as Array<Record<string, unknown>>) {
  console.log(Object.keys(o).join(","));
  console.log(Object.entries(o).map(([k]) => k).join(","));
  console.log(JSON.stringify(o));
  const fi: string[] = [];
  for (const k in o) fi.push(k);
  console.log(fi.join(","));
  console.log(Object.keys({ ...o }).join(","));
  console.log(Object.keys(Object.assign({}, o)).join(","));
  console.log(Object.getOwnPropertyNames(o).join(","));
  console.log(Object.values(o).length);
}
console.log(Object.keys(Thing.prototype).join(","), Object.getOwnPropertyNames(Thing.prototype).join(","));
console.log(Object.keys(Thing).join(","), Object.getOwnPropertyNames(Thing).filter((n) => n.startsWith("__")).length);
