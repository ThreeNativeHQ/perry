function make(name: string) {
  return class EngineClass {
    root: unknown;
    path: string;
    constructor(root: unknown, path: string) {
      this.root = root;
      this.path = path;
      Object.getOwnPropertyDescriptor(EngineClass.prototype, "bind");
    }
  };
}
export function bindEngine(): Record<string, unknown> {
  const classes: Record<string, any> = {};
  for (const name of ["Other", "PropertyBinding"]) {
    const C = make(name);
    Object.defineProperty(C, "name", { value: name });
    Object.defineProperty(C.prototype, "bind", { configurable: true, writable: true, value(this: any) { return "initial:" + this.path; } });
    Object.defineProperties(C.prototype, { bind: { configurable: true, writable: true, value(this: any) { return "bound:" + this.path; } },
      call: { configurable: true, writable: true, value(this: any, n: number) { return this.path + ":call:" + n; } },
      apply: { configurable: true, writable: true, value(this: any, n: number) { return this.path + ":apply:" + n; } }
    });
    classes[name] = C;
  }
  return classes;
}
