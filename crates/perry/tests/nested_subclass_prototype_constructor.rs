//! PRD-581 B6: constructor lookup follows the receiver's actual prototype chain.
use std::process::{Command, Stdio};

#[test]
fn constructor_on_created_and_adopted_nested_subclass_receivers() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("main.ts"),
        include_str!("../../../test-files/test_prd581_nested_subclass_constructor.ts"),
    )
    .unwrap();
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
    let run = Command::new(dir.path().join("main_bin"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8(run.stdout).unwrap(),
        concat!(
            "create function true\n",
            "prototype true\n",
            "new Tex\n",
            "adopt-object function true\n",
            "adopt-instance function true\n",
            "deep function true\n",
            "computed function true\n",
            "fresh true true\n",
            "captured true true true\n",
            "adopt-captured function true\n",
            "top-level function true\n",
            "function function true\n",
            "own-undefined true\n",
            "nearer true\n",
            "accessor true\n",
            "null true\n",
        )
    );
}
