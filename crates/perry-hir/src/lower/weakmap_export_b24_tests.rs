//! B24: a `new WeakMap()` local inside an exported function keeps the intrinsic
//! `js_weakmap_*` lowering, and a same-named binding typed as another
//! collection keeps ordinary dispatch.

#![cfg(test)]

use crate::Module;
use perry_diagnostics::SourceCache;

fn lower(src: &str) -> Module {
    let src = src.to_string();
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let mut cache = SourceCache::new();
            let parsed = perry_parser::parse_typescript_with_cache(&src, "weakmap.ts", &mut cache)
                .expect("parse should succeed");
            crate::lower_module(&parsed.module, "test", "weakmap.ts").expect("lower should succeed")
        })
        .expect("spawn lower thread")
        .join()
        .expect("lower thread panicked")
}

fn body_of(module: &Module, name: &str) -> String {
    let function = module
        .functions
        .iter()
        .find(|function| function.name == name)
        .expect("function is lowered");
    format!("{:?}", function.body)
}

#[test]
fn exported_function_weakmap_lowers_to_the_intrinsic() {
    let module = lower(
        r#"
export function make() {
  const cache = new WeakMap<object, number>();
  return (o: object) => {
    const v = cache.get(o);
    if (v === undefined) cache.set(o, 1);
    return v;
  };
}
export default function made() {
  const seen = new WeakSet<object>();
  return (o: object) => seen.has(o);
}
"#,
    );
    let make = body_of(&module, "make");
    assert!(make.contains("js_weakmap_get"), "{make}");
    assert!(make.contains("js_weakmap_set"), "{make}");
    assert!(body_of(&module, "made").contains("js_weakset_has"));
}

#[test]
fn a_same_named_typed_map_keeps_ordinary_dispatch() {
    let module = lower(
        r#"
function make() {
  const cache = new WeakMap<object, number>();
  return (o: object) => cache.get(o);
}
export function useMap(cache: Map<string, number>) {
  cache.set("a", 2);
  return cache.get("a");
}
make();
"#,
    );
    // Positive control: the WeakMap binding still lowers to the intrinsic.
    assert!(body_of(&module, "make").contains("js_weakmap_get"));
    let use_map = body_of(&module, "useMap");
    assert!(!use_map.contains("js_weakmap"), "{use_map}");
}
