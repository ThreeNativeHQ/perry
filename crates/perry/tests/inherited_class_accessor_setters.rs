//! PRD-581: class-slot stores must respect the receiver's actual prototype chain.
use std::process::{Command, Stdio};

fn run(source: &str, strict: bool) -> String {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.ts"), source).unwrap();
    if strict {
        std::fs::write(dir.path().join("package.json"), r#"{"type":"module"}"#).unwrap();
    }
    let compile = Command::new(env!("CARGO_BIN_EXE_perry"))
        .current_dir(dir.path())
        .env("PERRY_NO_TELEMETRY", "1")
        .env("PERRY_NO_AUTO_OPTIMIZE", "1")
        .env("CARGO_NET_OFFLINE", "true")
        .env_remove("PERRY_SAVE_LL")
        .stdin(Stdio::null())
        .args(["compile", "main.ts", "-o", "main_bin", "--no-cache"])
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let result = Command::new(dir.path().join("main_bin"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap()
}

#[test]
fn prototype_accessors_and_subclass_setter_strict_and_sloppy() {
    let source = include_str!("../../../test-files/test_prd581_inherited_class_setters.ts");
    for strict in [false, true] {
        assert_eq!(
            run(source, strict),
            "true true true\n".repeat(10),
            "strict={strict}"
        );
    }
}

#[test]
fn real_class_fields_and_own_data_shadow_inherited_accessors() {
    let source = r#"
let calls = 0;
class Base { x = 1; bump() { this.x += 1; } }
class Derived extends Base {
    get x() { return 99; }
    set x(v: number) { calls += 1; }
}
const value = new Derived(); value.bump();
console.log(value.x === 2, calls === 0, Object.keys(value).join(",") === "x");
function make() {
    class Bare { x: number; constructor() { this.x = 1; } bump() { this.x += 1; } }
    return Bare;
}
const Bare = make();
class Sub extends Bare { get x() { return 99; } set x(v: number) { calls += 1; } }
const bare = new Sub(); bare.bump();
console.log(bare.x === 2, calls === 0, Object.keys(bare).join(",") === "x");
const proto = Object.create(Base.prototype);
Object.defineProperty(proto, "x", { get: () => 99, set: (v: number) => { calls += 1; } });
const own = Object.create(proto);
Object.defineProperty(own, "x", { value: undefined, writable: true, configurable: true, enumerable: true });
own.x = 7;
console.log(own.x === 7, calls === 0, Object.keys(own).join(",") === "x");
"#;
    assert_eq!(run(source, true), "true true true\n".repeat(3));
}
