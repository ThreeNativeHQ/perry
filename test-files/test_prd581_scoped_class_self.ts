// PRD-581: each scoped class method must retain its evaluation's class value.
function make(seed: number) {
  class Vector3 {
    x = seed;
    clone() { return new Vector3(); }
    viaCtor() { return new (this.constructor as any)(); }
    self() { return Vector3; }
    shadow(Vector3: any) { return new Vector3(); }
    nested() { return (() => new Vector3())(); }
  }
  class Box { fresh() { return new Vector3(); } }
  const Vec3 = Vector3;
  return { Vector3, Box, alias: () => new Vec3() };
}
const a = make(1);
const b = make(2);
function check(math: any, seed: number) {
  const V = math.Vector3;
  const original = new V();
  for (const value of [original.clone(), original.viaCtor(), original.nested(), new math.Box().fresh(), math.alias()])
    console.log(Object.getPrototypeOf(value) === V.prototype, value.x === seed);
  console.log(original.self() === V, original.constructor === V);
  class Other { x = 99; }
  console.log(original.shadow(Other).x === 99);
  original.constructor = Other;
  console.log(original.clone().self() === V, original.viaCtor().x === 99);
}
check(a, 1);
check(b, 2);
console.log(a.Vector3 !== b.Vector3, a.Vector3.prototype !== b.Vector3.prototype);
class Top { clone() { return new Top(); } }
console.log(Object.getPrototypeOf(new Top().clone()) === Top.prototype, new Top().clone() instanceof Top);
