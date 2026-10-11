// PRD-582: a local a closure assigns is one binding shared by the closure and its function.
function counter(): number {
  let c = 0;
  const bump = () => {
    c = c + 1;
    return c;
  };
  bump();
  return bump() + c * 10;
}
console.log(`fn ${counter()}`);

function makeAcc(start: number) {
  let total = start;
  return {
    add: (n: number) => {
      total += n;
      return total;
    },
    read: () => total,
  };
}
const acc = makeAcc(5);
acc.add(2);
acc.add(3);
console.log(`acc ${acc.read()}`);

function postfix(): string {
  let i = 0;
  const tick = () => i++;
  const a = tick();
  const b = tick();
  return `${a} ${b} ${i}`;
}
console.log(`postfix ${postfix()}`);

const fns: (() => number)[] = [];
for (let k = 0; k < 3; k++) {
  let v = k;
  fns.push(() => {
    v = v * 2;
    return v;
  });
}
console.log(`each ${fns.map((f) => f()).join(",")}`);
