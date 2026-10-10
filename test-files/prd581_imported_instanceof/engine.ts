function exact(cls: { prototype: object }): void {
  Object.defineProperty(cls, Symbol.hasInstance, {
    configurable: true,
    value: (value: unknown) => {
      if (typeof value !== "object" || value === null) return false;
      for (let link = Object.getPrototypeOf(value); link !== null; link = Object.getPrototypeOf(link))
        if (link === cls.prototype) return true;
      return false;
    },
  });
}
function make(name: string): new () => object {
  return class EngineClass {
    tag = name;
  };
}
export function bind(): Record<string, unknown> {
  const classes: Record<string, new () => object> = {};
  for (const name of ["Object3D", "Mesh", "Group", "Scene"]) classes[name] = make(name);
  for (const [child, parent] of [
    ["Mesh", "Object3D"],
    ["Group", "Object3D"],
    ["Scene", "Object3D"],
  ])
    Object.setPrototypeOf(classes[child]!.prototype, classes[parent]!.prototype);
  const bound: Record<string, unknown> = { ...classes };
  for (const value of Object.values(bound))
    if (typeof value === "function" && typeof value.prototype === "object") exact(value as never);
  bound.wrapScene = () => Object.create(classes.Scene!.prototype);
  return bound;
}
