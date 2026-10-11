// k_math against perry-runtime compiled to wasm32: objects and arrays live in the runtime's
// linear memory; every field and element op is a Wasm-to-Wasm call into the runtime module
// (the static field-index lowering Perry's LLVM backend emits for declared class fields).
typedef unsigned int u32;
#define IMP(n) __attribute__((import_module("rt"), import_name(#n)))
IMP(js_object_alloc) void *js_object_alloc(u32 class_id, u32 field_count);
IMP(js_object_get_field_f64) double js_object_get_field_f64(void *o, u32 i);
IMP(js_object_set_field_f64) void js_object_set_field_f64(void *o, u32 i, double v);
IMP(js_array_alloc_with_length) void *js_array_alloc_with_length(u32 cap);
IMP(js_array_get_f64) double js_array_get_f64(void *a, u32 i);
IMP(js_array_set_f64) void js_array_set_f64(void *a, u32 i, double v);

#define G(o, i) js_object_get_field_f64(o, i)
#define S(o, i, v) js_object_set_field_f64(o, i, v)
#define AG(a, i) js_array_get_f64(a, i)
#define AS(a, i, v) js_array_set_f64(a, i, v)

static void compose(void *te, void *p, void *q, void *s) {
  double x = G(q, 0), y = G(q, 1), z = G(q, 2), w = G(q, 3);
  double x2 = x + x, y2 = y + y, z2 = z + z;
  double xx = x * x2, xy = x * y2, xz = x * z2, yy = y * y2, yz = y * z2, zz = z * z2;
  double wx = w * x2, wy = w * y2, wz = w * z2;
  double sx = G(s, 0), sy = G(s, 1), sz = G(s, 2);
  AS(te, 0, (1 - (yy + zz)) * sx); AS(te, 1, (xy + wz) * sx); AS(te, 2, (xz - wy) * sx); AS(te, 3, 0);
  AS(te, 4, (xy - wz) * sy); AS(te, 5, (1 - (xx + zz)) * sy); AS(te, 6, (yz + wx) * sy); AS(te, 7, 0);
  AS(te, 8, (xz + wy) * sz); AS(te, 9, (yz - wx) * sz); AS(te, 10, (1 - (xx + yy)) * sz); AS(te, 11, 0);
  AS(te, 12, G(p, 0)); AS(te, 13, G(p, 1)); AS(te, 14, G(p, 2)); AS(te, 15, 1);
}
static void applyMatrix4(void *v, void *e) {
  double x = G(v, 0), y = G(v, 1), z = G(v, 2);
  double w = 1 / (AG(e, 3) * x + AG(e, 7) * y + AG(e, 11) * z + AG(e, 15));
  S(v, 0, (AG(e, 0) * x + AG(e, 4) * y + AG(e, 8) * z + AG(e, 12)) * w);
  S(v, 1, (AG(e, 1) * x + AG(e, 5) * y + AG(e, 9) * z + AG(e, 13)) * w);
  S(v, 2, (AG(e, 2) * x + AG(e, 6) * y + AG(e, 10) * z + AG(e, 14)) * w);
}
static void *vec3(double x, double y, double z) { void *o = js_object_alloc(1, 3); S(o, 0, x); S(o, 1, y); S(o, 2, z); return o; }

__attribute__((export_name("run"))) double run(int n) {
  void *m = js_object_alloc(2, 1);
  void *te = js_array_alloc_with_length(16);
  for (u32 i = 0; i < 16; i++) AS(te, i, (i % 5) == 0 ? 1 : 0);
  (void)m;
  void *p = vec3(1, 2, 3), *s = vec3(1, 1, 1), *v = vec3(0, 0, 0);
  void *q = js_object_alloc(3, 4); S(q, 0, 0.1); S(q, 1, 0.2); S(q, 2, 0.3); S(q, 3, 0.927);
  double acc = 0;
  for (int i = 0; i < n; i++) {
    S(p, 0, i * 0.001);
    compose(te, p, q, s);
    S(v, 0, 1); S(v, 1, 2); S(v, 2, 3);
    applyMatrix4(v, te);
    acc += G(v, 0) + G(v, 1) + G(v, 2);
  }
  return acc;
}
