fn main() {
    let profile = std::env::var("PROFILE").unwrap_or_default();
    let is_dist = std::env::var("CARGO_DIST_VERSION").is_ok();
    if profile != "release" && !is_dist {
        return;
    }

    println!("cargo:rerun-if-changed=dashboard/src");
    println!("cargo:rerun-if-changed=dashboard/package.json");
    println!("cargo:rerun-if-changed=dashboard/svelte.config.js");
    println!("cargo:rerun-if-changed=dashboard/vite.config.js");

    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dashboard = format!("{}/dashboard", root);

    let node_modules = format!("{}/node_modules", dashboard);
    if std::env::var("CI").is_ok() && std::path::Path::new(&node_modules).exists() {
        std::fs::remove_dir_all(&node_modules)
            .unwrap_or_else(|e| eprintln!("Warning: could not remove node_modules: {}", e));
    }

    let lockfile = format!("{}/package-lock.json", dashboard);
    if std::env::var("CI").is_ok() && std::path::Path::new(&lockfile).exists() {
        std::fs::remove_file(&lockfile)
            .unwrap_or_else(|e| eprintln!("Warning: could not remove package-lock.json: {}", e));
    }

    run_npm(npm, &["install"], &dashboard);
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
