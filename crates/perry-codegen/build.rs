use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

fn source_files(path: &Path, files: &mut Vec<PathBuf>) {
    if !path.exists() {
        return;
    }
    if path.is_dir() {
        for child in std::fs::read_dir(path).expect("read codegen source directory") {
            source_files(&child.expect("read codegen source entry").path(), files);
        }
    } else {
        files.push(path.to_path_buf());
    }
}

fn fingerprint(files: &[(String, Vec<u8>)], settings: &[(String, String)]) -> u64 {
    let mut hash = DefaultHasher::new();
    "perry-codegen-object-v1".hash(&mut hash);
    files.hash(&mut hash);
    settings.hash(&mut hash);
    hash.finish()
}

#[cfg(not(test))]
fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let mut paths = Vec::new();
    for relative in [
        "src",
        "Cargo.toml",
        "build.rs",
        "../../Cargo.lock",
        "../perry-dispatch/src",
        "../perry-dispatch/Cargo.toml",
        "../perry-hir/src",
        "../perry-hir/Cargo.toml",
        "../perry-api-manifest/src",
        "../perry-api-manifest/Cargo.toml",
    ] {
        let path = root.join(relative);
        println!("cargo:rerun-if-changed={}", path.display());
        source_files(&path, &mut paths);
    }
    paths.sort();
    let files: Vec<_> = paths
        .into_iter()
        .map(|path| {
            let name = path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = std::fs::read(&path).expect("read codegen fingerprint input");
            (name, bytes)
        })
        .collect();
    let mut settings: Vec<_> = std::env::vars()
        .filter(|(key, _)| {
            key.starts_with("CARGO_FEATURE_")
                || key.starts_with("CARGO_CFG_")
                || matches!(
                    key.as_str(),
                    "CARGO_PKG_VERSION" | "CARGO_ENCODED_RUSTFLAGS"
                )
        })
        .collect();
    if let Some(prefix) = std::env::var_os("LLVM_SYS_221_PREFIX") {
        let config = PathBuf::from(prefix).join("bin/llvm-config");
        println!("cargo:rerun-if-changed={}", config.display());
        if config.is_file() {
            let version = std::process::Command::new(config)
                .arg("--version")
                .output()
                .expect("read LLVM version");
            assert!(version.status.success(), "LLVM version probe failed");
            settings.push((
                "LLVM_VERSION".into(),
                String::from_utf8(version.stdout).unwrap(),
            ));
        }
    }
    settings.sort();
    println!(
        "cargo:rustc-env=PERRY_CODEGEN_BUILD_ID={:016x}",
        fingerprint(&files, &settings)
    );
    println!("cargo:rerun-if-env-changed=LLVM_SYS_221_PREFIX");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let prefix = std::env::var_os("LLVM_SYS_221_PREFIX").unwrap_or_else(|| {
        panic!("LLVM_SYS_221_PREFIX must point to the LLVM 22 development archive on Windows")
    });
    let lib_dir = PathBuf::from(prefix).join("lib");
    if !lib_dir.join("LLVM-C.lib").is_file() {
        panic!("{} does not contain LLVM-C.lib", lib_dir.display());
    }
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=LLVM-C");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_fingerprint_tracks_emission_sources_and_flags() {
        let files = vec![("src/nanbox.rs".into(), b"codegen-v1".to_vec())];
        let flags = vec![("CARGO_FEATURE_LLVM_INPROCESS".into(), "1".into())];
        let id = fingerprint(&files, &flags);
        assert_eq!(id, fingerprint(&files, &flags));
        assert_ne!(
            id,
            fingerprint(&[("src/nanbox.rs".into(), b"codegen-v2".to_vec())], &flags)
        );
        assert_ne!(id, fingerprint(&files, &[]));
    }
}
