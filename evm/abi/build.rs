use std::env;

fn main() {
    let base_dir = env::current_dir()
        .expect("Failed to get current directory")
        .parent()
        .expect("Failed to get parent directory")
        .display()
        .to_string();

    println!("cargo:rerun-if-changed={base_dir}/src");
    println!("cargo:rerun-if-changed={base_dir}/foundry.toml");
    println!("cargo:rerun-if-changed={base_dir}/remappings.txt");
}
