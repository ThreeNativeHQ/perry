// PRD-582: statement forms that left an extra value on the Wasm stack, and
// update expressions that must store through the object exactly once.
const seen: number[] = [];
[1, 2, 3].forEach((n) => {
  seen.push(n * 2);
});
console.log(seen.join(","));

const item = { count: 2 };
console.log(++item.count);
console.log(item.count++);
console.log(--item.count);
console.log(item.count);

const values = [5, 7];
let reads = 0;
function at(): number {
  reads++;
  return 1;
}
console.log(values[at()]++);
console.log(++values[at()]);
console.log(`${values.join(",")} ${reads}`);

// Loose equality goes through the js_loose_eq bridge.
const looseValues: unknown[] = [undefined, 0, "1"];
console.log(looseValues.map((v) => `${v == null} ${v != 1}`).join(";"));

// Several arguments print on one line, separated by spaces.
console.log("multi", 1, true);
