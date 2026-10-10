//! PRD-581: mapped TypedArray factories preserve the constructor receiver.
use std::process::{Command, Stdio};

#[test]
fn typed_array_from_maps_values_indices_and_this_arg() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("main.ts"),
        include_str!("../../../test-files/test_prd581_typed_array_from_map.ts"),
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
    assert_eq!(String::from_utf8(run.stdout).unwrap(), EXPECTED);
}
const EXPECTED: &str = "A 4\nB 4\nE 4\nthisArg 3\nInt8Array true true\nUint8Array true true\nUint8ClampedArray true true\nInt16Array true true\nUint16Array true true\nInt32Array true true\nUint32Array true true\nFloat16Array true true\nFloat32Array true true\nFloat64Array true true\nBigInt64Array true true\nBigUint64Array true true\nreified 2 4\nno-map 0 4\narray-control 3\ninvalid true\ninvalid true\ninvalid true\nmapper-throw mapper\n";
