use std::process::Command;

const FRONTEND_DIR: &str = "frontend";

fn main() {
    let status = Command::new("npm")
        .args(["install"])
        .current_dir(FRONTEND_DIR)
        .status()
        .expect("Failed to run 'npm install'");

    assert!(status.success(), "'npm install' failed");

    let status = Command::new("npm")
        .args(["run", "build", "--", "--outDir", "../dist"])
        .current_dir(FRONTEND_DIR)
        .status()
        .expect("Failed to run 'npm run build'");

    assert!(status.success(), "'npm run build' failed");

    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/package.json");
}
