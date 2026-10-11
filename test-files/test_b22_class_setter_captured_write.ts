// B22: a class setter that writes a captured (closure) variable updates it for every observer.
function make(start: number) {
  let counter = start;
  let log = "";
  class Box {
    get count(): number {
      return counter;
    }
    set count(value: number) {
      counter = value;
    }
    static set tag(value: string) {
      log += value;
    }
    static get tag(): string {
      return log;
    }
    bump(): void {
      counter += 1;
    }
    setMethod(value: number): void {
      counter = value;
    }
  }
  return { Box, read: () => counter };
}
const { Box, read } = make(1);
const b = new Box();
console.log(read(), b.count);
b.count = 5;
console.log(read(), b.count);
b.bump();
console.log(read(), b.count);
b.setMethod(40);
console.log(read(), b.count);
b.count = 7;
console.log(read(), b.count);
Box.tag = "a";
Box.tag = "b";
console.log(Box.tag);
