use std::process::Command;

fn main() {
    emit_build_info();
    tauri_build::build();
}

fn emit_build_info() {
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");

    let branch = git_output(&["branch", "--show-current"]).unwrap_or_else(|| "unknown".to_string());
    if branch != "unknown" {
        println!("cargo:rerun-if-changed=../.git/refs/heads/{branch}");
    }

    let commit =
        git_output(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let dirty = git_output(&["status", "--short"])
        .map(|status| !status.is_empty())
        .unwrap_or(false);
    let built_at = command_output("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=PIFORMA_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=PIFORMA_BUILD_BRANCH={branch}");
    println!("cargo:rustc-env=PIFORMA_BUILD_DIRTY={dirty}");
    println!("cargo:rustc-env=PIFORMA_BUILD_BUILT_AT={built_at}");
}

fn git_output(args: &[&str]) -> Option<String> {
    command_output("git", args)
}

fn command_output(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
