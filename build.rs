use std::{env, process::Command};

use cargo_metadata::MetadataCommand;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    set_donation_link();
    set_build_version();
}

fn set_donation_link() {
    let meta = MetadataCommand::new()
        .manifest_path("./Cargo.toml")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .exec()
        .unwrap();

    let root = meta.root_package().unwrap();
    let link = root.metadata["donate"].as_str().unwrap();
    println!("cargo:rustc-env=DONATION_LINK={}", link);
}

fn get_git_version() -> Option<String> {
    let commit_hash = Command::new("git")
        .args(["rev-parse", "--short=10", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| {
            String::from_utf8(output.stdout)
                .ok()
                .map(|s| s.trim().to_string())
        });

    let is_dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map(|out| !out.stdout.is_empty())
        .unwrap_or(false);

    let dirty_suffix = if is_dirty { "-dirty" } else { "" };
    Some(format!("{}{}", commit_hash?, dirty_suffix))
}

fn set_build_version() {
    let git_version = get_git_version();
    let version = env!("CARGO_PKG_VERSION").to_string();

    let mut full_version = version.clone();

    if let Some(git) = git_version {
        full_version.push_str(&format!(" (commit {git})"));
    }

    println!("cargo:rustc-env=BUILD_VERSION={}", full_version);
}
