class Priv { #p = 1; q = 2; peek() { return this.#p; } }
const pv = new Priv();
console.log(JSON.stringify({ ...new Priv() }), JSON.stringify({ ...pv }), JSON.stringify(Object.assign({}, pv)));

const publicSpelling = { "#p": 3, "#<perry:private-value:1:#p>": 4 };
console.log(JSON.stringify({ ...publicSpelling }));
console.log(Object.keys(pv).join(","), Object.getOwnPropertyNames(pv).join(","));

console.log("assigned-keys", Object.keys(Object.assign({}, pv)).join(","));
class PrivateWithMutation { first = 1; second = 2; #p = 3; q = 4; }
const mutating: any = new PrivateWithMutation();
Object.defineProperty(mutating, "first", { enumerable: true, get() { delete mutating.second; return 1; } });
const mutationClone = { ...mutating };
console.log("mutation", JSON.stringify(mutationClone), Object.keys(mutationClone).filter(k => k.startsWith("#")).length);
