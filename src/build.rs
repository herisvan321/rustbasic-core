use std::fs;
use std::collections::HashSet;

pub fn run() {
    // Tell Cargo to re-run this build script if Cargo.toml changes
    println!("cargo:rerun-if-changed=Cargo.toml");

    let mut detected_packages = HashSet::new();

    let mut in_dependencies_section = false;

    if let Ok(toml_content) = fs::read_to_string("Cargo.toml") {
        for line in toml_content.lines() {
            let line = line.trim();
            // Ignore commented out lines
            if line.starts_with('#') {
                continue;
            }
            
            // Track sections
            if line.starts_with('[') && line.ends_with(']') {
                let section = &line[1..line.len() - 1];
                in_dependencies_section = section == "dependencies" 
                    || section.starts_with("dependencies.")
                    || section == "dev-dependencies"
                    || section == "build-dependencies";
                continue;
            }
            
            if !in_dependencies_section {
                continue;
            }
            
            if let Some(pos) = line.find("rustbasic-") {
                let start_idx = pos + "rustbasic-".len();
                let mut end_idx = start_idx;
                let chars: Vec<char> = line.chars().collect();
                while end_idx < chars.len() {
                    let c = chars[end_idx];
                    if c.is_alphanumeric() || c == '_' || c == '-' {
                        end_idx += 1;
                    } else {
                        break;
                    }
                }
                
                if end_idx > start_idx {
                    let suffix: String = chars[start_idx..end_idx].iter().collect();
                    // Skip core and cli packages, and collect the rest
                    if suffix != "core" && suffix != "cli" && !suffix.is_empty() {
                        detected_packages.insert(suffix);
                    }
                }
            }
        }
    }

    // Automatically register and enable cfg for each detected package
    for suffix in &detected_packages {
        println!("cargo:rustc-check-cfg=cfg({})", suffix);
        println!("cargo:rustc-cfg={}", suffix);
    }

    // Always declare known optional package cfg names to avoid compiler warnings
    if !detected_packages.contains("breeze") {
        println!("cargo:rustc-check-cfg=cfg(breeze)");
    }

    if let Ok(target) = std::env::var("TARGET") && target.contains("android") {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        println!("cargo:rustc-link-search=native={}/target/{}/sqlite", manifest_dir, target);
    }
}
