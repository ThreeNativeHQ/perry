// B24: weak wrappers reached through dynamic receivers keep JS semantics.
class Cache {
  #seen = new WeakMap<object, number>();
  readonly tags = new WeakSet<object>();
  readonly refs = new Map<string, WeakRef<object>>();
  note(key: object, value: number): number | undefined {
    const before = this.#seen.get(key);
    this.#seen.set(key, value);
    return before;
  }
  has(key: object): boolean {
    return this.#seen.has(key);
  }
  forget(key: object): boolean {
    return this.#seen.delete(key);
  }
}
const cache = new Cache();
const a = { a: 1 };
const b = { b: 2 };
console.log(cache.note(a, 1), cache.note(a, 2), cache.has(a), cache.has(b));
console.log(cache.forget(a), cache.forget(a), cache.has(a));
cache.tags.add(b);
console.log(cache.tags.has(b), cache.tags.has(a), cache.tags.delete(b), cache.tags.has(b));
cache.refs.set("b", new WeakRef(b));
console.log(cache.refs.get("b")?.deref() === b, cache.refs.get("x")?.deref());

const own = [new WeakMap<object, number>()][0] as WeakMap<object, number>;
(own as unknown as { get: () => number }).get = () => 42;
console.log(own.get(a));

const accessor = [new WeakRef(a)][0] as WeakRef<object>;
Object.defineProperty(accessor, "deref", { get: () => () => "accessor" });
console.log(accessor.deref());

const refs: WeakRef<object>[] = [new WeakRef(b)];
let hits = 0;
for (let i = 0; i < 1000; i++) if (refs[0]?.deref() === b) hits++;
console.log(hits);
const registry = [new FinalizationRegistry<string>(() => {})][0] as FinalizationRegistry<string>;
const token = {};
registry.register({}, "held", token);
console.log(registry.unregister(token), registry.unregister(token));
