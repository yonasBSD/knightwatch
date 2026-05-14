fn main() {
    if std::env::var("PROFILE").unwrap_or_default() != "release" {
        return;
    }
    println!("cargo:rerun-if-changed=dashboard/src");
    println!("cargo:rerun-if-changed=dashboard/package.json");
    println!("cargo:rerun-if-changed=dashboard/svelte.config.js");
    println!("cargo:rerun-if-changed=dashboard/vite.config.js");
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dashboard = format!("{}/dashboard", root);
    if !std::path::Path::new(&format!("{}/node_modules", dashboard)).exists() {
        run_npm(npm, &["install"], &dashboard);
    }
    run_npm(npm, &["run", "build"], &dashboard);
}

fn run_npm(npm: &str, args: &[&str], cwd: &str) {
    let status = std::process::Command::new(npm)
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap_or_else(|e| panic!("Failed to spawn npm {:?}: {}", args, e));
    if !status.success() {
        panic!("npm {:?} failed with status: {}", args, status);
    }
}
