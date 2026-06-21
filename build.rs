fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if let Ok(target) = std::env::var("TARGET") {
        if target.contains("android") {
            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
            
            // Check in current manifest dir target (e.g. if compiling inside rustbasic-core)
            println!("cargo:rustc-link-search=native={}/target/{}/sqlite", manifest_dir, target);
            
            // Check in sibling rustbasic target (standard starterkit application path)
            println!("cargo:rustc-link-search=native={}/../rustbasic/target/{}/sqlite", manifest_dir, target);
        }
    }
}
