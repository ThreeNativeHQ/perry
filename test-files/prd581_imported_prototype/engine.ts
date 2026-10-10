export class Material {
  static marker = 581;
  onBeforeCompile() { return "original"; }
}
export class Alternate {
  static marker = 582;
  onBeforeCompile() { return "facade"; }
}
export function bindEngine(): Record<string, unknown> { return { Material, Alternate }; }
