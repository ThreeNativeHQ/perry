//! PRD-581: imported runtime constructors and factory-local class identity.
use std::path::Path;
use std::process::{Command, Stdio};

fn compile_and_run(dir: &Path, source: &str) -> String {
    std::fs::write(dir.join("main.ts"), source).unwrap();
    let compile = Command::new(env!("CARGO_BIN_EXE_perry"))
        .current_dir(dir)
        .env("PERRY_NO_TELEMETRY", "1")
        .env("PERRY_NO_AUTO_OPTIMIZE", "1")
        .env("CARGO_NET_OFFLINE", "true")
        .env_remove("PERRY_SAVE_LL")
        .stdin(Stdio::null())
        .args([
            "compile",
            "main.ts",
            "-o",
            "main_bin",
            "--no-cache",
            "--trace",
            "llvm",
        ])
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new(dir.join("main_bin"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8(run.stdout).unwrap()
}

#[test]
fn imported_runtime_value_real_class_and_local_class() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("engine.ts"),
        include_str!("../../../test-files/prd581_imported_instanceof/engine.ts"),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("three.ts"),
        include_str!("../../../test-files/prd581_imported_instanceof/three.ts"),
    )
    .unwrap();
    let stdout = compile_and_run(
        dir.path(),
        include_str!("../../../test-files/prd581_imported_instanceof/main.ts"),
    );
    assert_eq!(stdout, "any (want true) true\nlocal unknown->any (want true) true\nraw unknown (want true) true\nctor cast (want true) true\nruntime true false\ndefault true false\nimported true false\nlocal true false\nbuiltins true true true\n");
    let llvm: String = std::fs::read_dir(dir.path().join(".perry-trace/llvm"))
        .unwrap()
        .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect();
    for (name, dynamic) in [
        ("runtimeCheck", true),
        ("importedCheck", false),
        ("localCheck", false),
    ] {
        let body = llvm
            .split("\n}")
            .find(|body| {
                body.lines()
                    .any(|line| line.starts_with("define ") && line.contains(&format!("{name}(")))
            })
            .expect("check function emitted");
        let call = if dynamic {
            "@js_instanceof_dynamic("
        } else {
            "@js_instanceof("
        };
        assert!(body.contains(call), "{name} must use {call}: {body}");
        if !dynamic {
            assert!(
                !body.contains("@js_instanceof_dynamic("),
                "static class path changed: {body}"
            );
        }
    }
}

#[test]
fn scoped_class_self_constructor_and_repeated_factory() {
    let dir = tempfile::tempdir().unwrap();
    let stdout = compile_and_run(
        dir.path(),
        include_str!("../../../test-files/test_prd581_scoped_class_self.ts"),
    );
    assert_eq!(
        stdout,
        "true true\n".repeat(6)
            + "true\ntrue true\n"
            + &"true true\n".repeat(6)
            + "true\ntrue true\ntrue true\ntrue true\n"
    );
}

#[test]
fn named_class_expression_self_constructs_its_own_evaluation() {
    let dir = tempfile::tempdir().unwrap();
    let stdout = compile_and_run(
        dir.path(),
        r#"
function make(seed: number) {
    return class Self {
        x = seed;
        clone() { return new Self(); }
    };
}
const A = make(1);
const B = make(2);
const a = new A().clone();
const b = new B().clone();
console.log(Object.getPrototypeOf(a) === A.prototype, a.x === 1);
console.log(Object.getPrototypeOf(b) === B.prototype, b.x === 2);
"#,
    );
    assert_eq!(stdout, "true true\ntrue true\n");
}

#[test]
fn scoped_self_binding_does_not_shadow_explicit_global_constructor() {
    let dir = tempfile::tempdir().unwrap();
    let stdout = compile_and_run(
        dir.path(),
        r#"
function make(seed: number) {
    class Map {
        x = seed;
        clone() { return new Map(); }
        builtin() { return new globalThis.Map(); }
    }
    return Map;
}
const C = make(7);
const value = new C();
console.log(value.clone().x === 7);
console.log(value.builtin().size === 0, typeof value.builtin().set === "function");
"#,
    );
    assert_eq!(stdout, "true\ntrue true\n");
}

#[test]
fn inlined_scoped_array_class_retains_self_owner() {
    let dir = tempfile::tempdir().unwrap();
    let stdout = compile_and_run(
        dir.path(),
        r#"
function make(seed: number) {
    class Array {
        x = seed;
        clone() { return new Array(); }
    }
    return Array;
}
const C = make(7);
const clone = new C().clone();
console.log(clone.x === 7, Object.getPrototypeOf(clone) === C.prototype);
"#,
    );
    assert_eq!(stdout, "true true\n");
}
