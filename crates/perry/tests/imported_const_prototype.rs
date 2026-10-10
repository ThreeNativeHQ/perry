//! PRD-581: imported facade constructor properties keep runtime value tags.
use std::process::{Command, Stdio};

fn compile_and_run(source: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("engine.ts"),
        include_str!("../../../test-files/prd581_imported_prototype/engine.ts"),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("three.ts"),
        include_str!("../../../test-files/prd581_imported_prototype/three.ts"),
    )
    .unwrap();
    std::fs::write(dir.path().join("main.ts"), source).unwrap();
    std::fs::write(
        dir.path().join("rebound.ts"),
        include_str!("../../../test-files/prd581_imported_prototype/rebound.ts"),
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
    String::from_utf8(run.stdout).unwrap()
}

#[test]
fn imported_const_constructor_prototype_is_an_object() {
    let stdout = compile_and_run(include_str!(
        "../../../test-files/prd581_imported_prototype/main.ts"
    ));
    assert_eq!(stdout, "direct object\ncast object\ncomputed object\nalias object\nidentity true true\nmethod original\nother-name object\nstatic 581\nplain 7\n");
}

#[test]
fn imported_const_wins_over_equal_named_class_metadata() {
    let stdout = compile_and_run(include_str!(
        "../../../test-files/prd581_imported_prototype/collision.ts"
    ));
    assert_eq!(
        stdout,
        "identity false true\nmarker 582 582\nprototype false true\ninstance facade\n"
    );
}
