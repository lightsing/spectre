#[cfg(target_os = "macos")]
fn main() {
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
    println!("cargo:rustc-link-lib=framework=Security");
}

#[cfg(not(target_os = "macos"))]
fn main() {}