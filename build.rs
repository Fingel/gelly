use std::path::PathBuf;
use std::process::Command;

fn main() {
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/resources.gresource.xml",
        "gelly.gresource",
    );

    let locale_dir = std::env::var("LOCALEDIR").unwrap_or_else(|_| {
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let dir = format!("{}/locale", out_dir);
        compile_po_files(&dir);
        dir
    });

    println!("cargo:rustc-env=LOCALEDIR={}", locale_dir);
}

fn compile_po_files(locale_dir: &str) {
    let po_dir = PathBuf::from("po");
    if !po_dir.exists() {
        return;
    }

    println!("cargo:rerun-if-changed=po/");

    for entry in std::fs::read_dir(&po_dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("po") {
            continue;
        }

        let lang = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        let mo_dir = PathBuf::from(locale_dir)
            .join(&lang)
            .join("LC_MESSAGES");
        std::fs::create_dir_all(&mo_dir).expect("Failed to create locale dir");

        let status = Command::new("msgfmt")
            .arg("-o")
            .arg(mo_dir.join("gelly.mo"))
            .arg(&path)
            .status()
            .expect("Failed to run msgfmt (is gettext installed?)");

        assert!(status.success(), "msgfmt failed for {}", path.display());
        println!("cargo:rerun-if-changed={}", path.display());
    }
}
