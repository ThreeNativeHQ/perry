class Budget {
  #value = 7;
  read(): number { return this.#value; }
  static has(o: any): boolean { return #value in o; }
}
const buffer = Buffer.alloc(8);
const typed = new Float32Array(8);
const budget = new Budget();
const backend: any = { step: () => 7, spare: () => 3, other: () => 5 };
const n = 10000000;
let sum = 0;
let start = Date.now();
for (let i = 0; i < n; i++) sum += budget.read();
console.log('private', (Date.now() - start) * 1000000 / n, sum);
sum = 0;
start = Date.now();
for (let i = 0; i < n; i++) sum += backend.step();
console.log('closures', (Date.now() - start) * 1000000 / n, sum);
console.log('brands', Budget.has(budget), Budget.has(buffer), Budget.has(typed));
