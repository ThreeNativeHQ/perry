// Helper module for test_prd582_wasm_imports.ts: exported `const` functions, as three/tsl ships them.
export const vec2 = (x: number, y: number): string => `vec2(${x}, ${y})`;
export const vec4 = (x: number, y: number, z: number, w: number): string => `vec4(${x}, ${y}, ${z}, ${w})`;
