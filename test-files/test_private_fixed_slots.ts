// Fixed slots must preserve initialization order and public offsets.
class SlotBase {
    #x = 3;
    publicValue = 7;
    #first = this.beforeSecond();
    #second = 11;
    beforeSecond(): boolean { return #second in this; }
    read(): number { return this.#x; }
    write(n: number): void { this.#x = n; }
    early(): boolean { return this.#first; }
    static has(o: any): boolean { return #x in o; }
    static readForeign(o: any): number { return o.#x; }
}
class SlotChild extends SlotBase { childValue = 13; #own = 17; own(): number { return this.#own; } }
const child = new SlotChild();
console.log(child.read(), child.publicValue, child.childValue, child.own(), child.early());
Object.freeze(child);
child.write(19);
console.log(child.read(), SlotBase.has(child), Object.keys(child).join(','));
for (const foreign of [{}, new Uint8Array(8), new ArrayBuffer(8), new DataView(new ArrayBuffer(8))]) {
    console.log(SlotBase.has(foreign));
    try { SlotBase.readForeign(foreign); console.log('unexpected'); } catch (e) { console.log(e instanceof TypeError); }
}
class Returning { constructor(o: any) { return o; } }
class Stamp extends Returning {
    #x = 23;
    static read(o: any): number { return o.#x; }
    static has(o: any): boolean { return #x in o; }
}
const plain: any = {};
new Stamp(plain);
console.log(Stamp.read(plain), Stamp.has(plain), Object.keys(plain).length);
try { new Stamp(plain); console.log('unexpected'); } catch (e) { console.log(e instanceof TypeError); }
const proxy: any = new Proxy({}, { set() { console.log('unexpected trap'); return false; } });
new Stamp(proxy);
console.log(Stamp.read(proxy), Stamp.has(proxy));

// A tagged private value remains visible to the collector across an allocating RHS.
class PointerSlot {
    #value: { n: number } = { n: 29 };
    replace(): void { this.#value = this.allocate(); }
    allocate(): { n: number } {
        for (let i = 0; i < 20000; i++) { const trash = { i, text: 'allocation' + i }; }
        return { n: this.#value.n + 2 };
    }
    read(): number { return this.#value.n; }
}
const pointer = new PointerSlot(); pointer.replace(); console.log(pointer.read());

// PutValue checks a simple write's brand after evaluating its RHS.
let rhsRan = 0;
class WriteOrder { #x = 1; put(o: any): void { o.#x = ++rhsRan; } }
try { new WriteOrder().put({}); } catch (e) { console.log(e instanceof TypeError, rhsRan); }
class MixedPrivate {
    publicValue = 37; #x = 41;
    #add(n: number): number { return this.#x + n; }
    get #current(): number { return this.#x; }
    set #current(n: number) { this.#x = n; }
    read(): number { return this.#add(2) + this.#current; }
    write(n: number): void { this.#current = n; }
}
const mixed = new MixedPrivate();
console.log(mixed.read(), mixed.publicValue, Object.keys(mixed).join(','));
Object.freeze(mixed); mixed.write(43); console.log(mixed.read());
