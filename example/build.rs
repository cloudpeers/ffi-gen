use ffi_gen::FfiGen;
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let path = dir.join("api.rsh");
    println!(
        "cargo:rerun-if-changed={}",
        path.as_path().to_str().unwrap()
    );
    let ffigen = FfiGen::new(&path).unwrap();
    let dart = dir.join("dart").join("lib").join("bindings.dart");
    ffigen.generate_dart(dart, "api", "api").unwrap();
    let js = dir.join("js").join("bindings.mjs");
    ffigen.generate_js(js).unwrap();
    let ts = dir.join("js").join("bindings.d.ts");
    ffigen.generate_ts(ts).unwrap();
}
