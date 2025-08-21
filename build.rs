use std::process::Command;

fn main() {
    let frontend_dir = "frontend";

    println!("cargo:rerun-if-changed={frontend_dir}/public");
    println!("cargo:rerun-if-changed={frontend_dir}/src");
    println!("cargo:rerun-if-changed={frontend_dir}/index.html");
    println!("cargo:rerun-if-changed={frontend_dir}/jsconfig.json");
    println!("cargo:rerun-if-changed={frontend_dir}/package.json");
    println!("cargo:rerun-if-changed={frontend_dir}/svelte.config.js");
    println!("cargo:rerun-if-changed={frontend_dir}/vite.config.js");

    let status = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(frontend_dir)
        .status()
        .expect("Failed to execute npm build");

    if !status.success() {
        panic!("Failed to build frontend");
    }
}
