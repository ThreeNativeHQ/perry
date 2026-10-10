use std::process::{Command, Output, Stdio};

fn compile(dir: &std::path::Path, src: &str, strict: bool, object: bool) -> Output {
    std::fs::write(dir.join("x.ts"), src).expect("write fixture");
    let mut command = Command::new(env!("CARGO_BIN_EXE_perry"));
    command
        .current_dir(dir)
        .env("PERRY_NO_TELEMETRY", "1")
        .env("CARGO_NET_OFFLINE", "true")
        .env_remove("PERRY_ALLOW_UNIMPLEMENTED")
        .stdin(Stdio::null())
        .args([
            "compile",
            "x.ts",
            "-o",
            "x",
            "--no-cache",
            "--debug-symbols",
        ]);
    if strict {
        command.arg("--strict-unimplemented");
    }
    if object {
        command.arg("--no-link");
    }
    command.output().expect("run compiler")
}

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn strict_object_contains_no_function_or_class_source_registration() {
    let dir = tempfile::tempdir().expect("isolated fixture directory");
    let marker = "x=>{\"use strict\";\n}";
    let class = "class SourceMarker { method(){return 581} }";
    let src = format!("{marker}; {class}; console.log('ordinary literal');");
    let output = compile(dir.path(), &src, true, true);
    success(&output);
    let object = std::fs::read(dir.path().join("x")).expect("object bytes");
    for text in [
        marker,
        class,
        "js_register_function_source",
        "js_register_class_source",
    ] {
        assert!(
            !object
                .windows(text.len())
                .any(|bytes| bytes == text.as_bytes()),
            "retained {text}"
        );
    }
    assert!(object
        .windows("ordinary literal".len())
        .any(|bytes| bytes == b"ordinary literal"));
}

#[test]
fn callable_to_string_fails_strict_and_prints_original_source_non_strict() {
    let dir = tempfile::tempdir().expect("isolated fixture directory");
    let src = "function f(){return 7} console.log(f.toString());";
    let refused = compile(dir.path(), src, true, false);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("ERR_STRICT_SOURCE_REFLECTION"));
    let ordinary = compile(dir.path(), src, false, false);
    success(&ordinary);
    let run = Command::new(dir.path().join("x"))
        .stdin(Stdio::null())
        .output()
        .expect("run binary");
    success(&run);
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "function f(){return 7}\n"
    );
}
