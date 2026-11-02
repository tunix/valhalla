use std::{
    env, fs,
    path::PathBuf,
    process::{Command, Stdio},
};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));

    // 1. Compile Blueprint markup into GtkBuilder XML.
    let src = manifest.join("resources").join("window.blp");
    let dest = out.join("window.ui");
    println!("cargo:rerun-if-changed={}", src.display());
    let file = fs::File::create(&dest).expect("failed to create ui file");
    let status = Command::new("blueprint-compiler")
        .arg("compile")
        .arg(&src)
        .stdout(Stdio::from(file))
        .status()
        .expect("blueprint-compiler not found; install it (dnf install blueprint-compiler)");
    assert!(status.success(), "blueprint-compiler failed on window.blp");

    // 2. Compile the GSettings schema for the portable artifacts. GLib
    //    never reads schemas from a GResource: the default schema source
    //    only looks for compiled `gschemas.compiled` files under the XDG
    //    data dirs and GSETTINGS_SCHEMA_DIR. The tarball/AppImage ship
    //    gschemas.compiled next to the binary; main.rs points
    //    GSETTINGS_SCHEMA_DIR there when the exe dir carries one.
    let schema = "io.github.tunix.valhalla.gschema.xml";
    fs::copy(manifest.join("data").join(schema), out.join(schema)).expect("copy gschema");
    println!("cargo:rerun-if-changed=data/{schema}");
    // glib-compile-schemas writes gschemas.compiled into the directory it
    // scans (no --target on older GLib, e.g. Ubuntu 22.04's 2.72), so
    // compile OUT_DIR itself — the only *.gschema.xml there is ours.
    let status = Command::new("glib-compile-schemas")
        .arg(&out)
        .status()
        .expect("glib-compile-schemas not found");
    assert!(status.success(), "glib-compile-schemas failed");

    // 2b. Drop the compiled schema next to the binary (target/<profile>/)
    //     so release packaging ships it and dev runs find it via the
    //     exe-dir lookup in main.rs. OUT_DIR is
    //     <target>/<profile>/build/<pkg>-<hash>/out; only treat the copy
    //     as valid when that layout holds (it can be redirected).
    let mut ancestors = out.ancestors();
    let _out = ancestors.next();
    let _pkg = ancestors.next();
    let build_dir = ancestors.next();
    let profile_dir = ancestors.next();
    if let (Some(build_dir), Some(profile_dir)) = (build_dir, profile_dir) {
        if build_dir.file_name().is_some_and(|name| name == "build") {
            fs::copy(
                out.join("gschemas.compiled"),
                profile_dir.join("gschemas.compiled"),
            )
            .expect("copy gschemas.compiled next to the binary");
        }
    }

    // 3. Generate and compile the GResource bundle.
    let icon_dir = manifest.join("data/icons/hicolor/scalable/apps");
    println!(
        "cargo:rerun-if-changed={}",
        icon_dir.join("io.github.tunix.valhalla.svg").display()
    );
    // The heart toggle icon ships EMBEDDED: the theme lookup for
    // emblem-favorite-symbolic fails inside some sandboxes (flatpak without
    // a matching icon theme), and GTK then renders the emoji fallback —
    // which CSS recoloring cannot touch (stays red permanently).
    // NOTE: it must live under apps/ — GTK's resource-based icon theme
    // lookup resolves the app's own embedded icons only from the apps dir
    // (the app icon already ships there and works).
    let action_dir = icon_dir.clone();
    let heart = "valhalla-heart-symbolic.svg";
    println!(
        "cargo:rerun-if-changed={}",
        action_dir.join(heart).display()
    );
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><gresources><gresource prefix="/io/github/tunix/valhalla">"#,
    );
    xml.push_str(r#"<file compressed="true" alias="window.ui">window.ui</file>"#);
    xml.push_str(
        r#"<file compressed="true" alias="icons/hicolor/scalable/apps/io.github.tunix.valhalla.svg">io.github.tunix.valhalla.svg</file>"#,
    );
    xml.push_str(&format!(
        r#"<file compressed="true" alias="icons/hicolor/scalable/apps/{heart}">{heart}</file>"#,
    ));
    // Brand logos for the Preferences sidebar (per-source pages). Full-color,
    // embedded for the same sandbox reason as the heart icon.
    let sources_dir = icon_dir.clone(); // logos live in apps/ alongside the heart icon
    println!("cargo:rerun-if-changed={}", sources_dir.display());
    for entry in std::fs::read_dir(&sources_dir).expect("read sources icon dir") {
        let path = entry.expect("source icon entry").path();
        if path.extension().and_then(|e| e.to_str()) == Some("svg") {
            let name = path.file_name().unwrap().to_str().unwrap().to_string();
            if name == heart || name == "io.github.tunix.valhalla.svg" {
                continue; // already listed above
            }
            xml.push_str(&format!(
                r#"<file compressed="true" alias="icons/hicolor/scalable/apps/{name}">{name}</file>"#,
            ));
        }
    }
    xml.push_str(r#"</gresource></gresources>"#);
    let gxml = out.join("valhalla.gresources.xml");
    fs::write(&gxml, xml).expect("write gresource xml");

    glib_build_tools::compile_resources(
        &[out, icon_dir, action_dir, sources_dir],
        gxml.to_str().unwrap(),
        "valhalla.gresource",
    );

    // 5. Install the schema into the user schema dir so dev runs (outside
    //    Flatpak) find it via dconf.
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = env::var_os("HOME").map(PathBuf::from).expect("HOME");
            home.join(".local/share")
        });
    let user_schema_dir = data_home.join("glib-2.0/schemas");
    fs::create_dir_all(&user_schema_dir).expect("create user schema dir");
    let installed = user_schema_dir.join(schema);
    let changed = fs::read(&installed)
        .map(|old| old != fs::read(manifest.join("data").join(schema)).unwrap())
        .unwrap_or(true);
    if changed {
        fs::copy(manifest.join("data").join(schema), &installed).expect("install gschema");
        let status = Command::new("glib-compile-schemas")
            .arg(&user_schema_dir)
            .status()
            .expect("glib-compile-schemas not found");
        assert!(status.success(), "glib-compile-schemas failed");
    }
}
