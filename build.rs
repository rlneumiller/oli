fn main() {
    // Notify cargo to re-run this script if these files change
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // We don't need to explicitly link the tree-sitter libraries because
    // they get linked automatically as dependencies. Instead, we just need to
    // ensure these dependencies are properly listed in Cargo.toml.

    // However, we do need to ensure certain system dependencies are available

    // On macOS, link against the C++ standard library
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=c++");
    }

    // On Linux, link against the C++ standard library
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=stdc++");
    }

    // On Windows, link against the Microsoft C++ runtime
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib=dylib=msvcrt");
    }

    // Let cargo know we're going to use tree-sitter's parser feature
    println!("cargo:rustc-cfg=feature=\"parser\"");
}
