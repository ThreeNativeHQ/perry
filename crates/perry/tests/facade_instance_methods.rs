//! PRD-581: facade instance methods win over Function prototype shortcuts.
use std::process::{Command, Stdio};

#[test]
fn facade_instance_bind_call_apply_resolve_the_prototype_method() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("engine.ts"),
        include_str!("../../../test-files/prd581_facade_methods/engine.ts"),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("three.ts"),
        include_str!("../../../test-files/prd581_facade_methods/three.ts"),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("main.ts"),
        include_str!("../../../test-files/prd581_facade_methods/main.ts"),
    )
    .unwrap();
    let compile = Command::new(env!("CARGO_BIN_EXE_perry"))
        .current_dir(dir.path())
        .env("PERRY_NO_TELEMETRY", "1")
        .env("PERRY_NO_AUTO_OPTIMIZE", "1")
        .stdin(Stdio::null())
        .args(["compile", "main.ts", "-o", "main_bin", "--no-cache"])
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new(dir.path().join("main_bin"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8(run.stdout).unwrap(), "bind bound:a.b\ncall a.b:call:7\napply a.b:apply:8\nprototype true true\nexplicit bound:a.b\nown own:a.b\ninvalid true\nfunctions 5 5 5\nbrand true\n");
}
