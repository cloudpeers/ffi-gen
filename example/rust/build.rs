use ffi_gen::{DartGenerator, Interface, JsGenerator, TsGenerator};
use std::process::Command;

fn main() {
    let path = "./api.rsh";
    println!("cargo:rerun-if-changed={}", path);
    let s = std::fs::read_to_string(path).unwrap();
    let iface = Interface::parse(&s).unwrap();
    let dart = DartGenerator::new("api".to_string());
    let js = JsGenerator::default();
    let ts = TsGenerator::default();
    let dart = dart.generate(iface.clone()).to_file_string().unwrap();
    let js = js.generate(iface.clone()).to_file_string().unwrap();
    let ts = ts.generate(iface).to_file_string().unwrap();

    std::fs::write("../dart/lib/bindings.dart", &dart).unwrap();
    std::fs::write("../js/bindings.js", &js).unwrap();
    std::fs::write("../js/bindings.d.ts", &ts).unwrap();

    let ret = Command::new("dart")
        .arg("format")
        .arg("../dart/lib/bindings.dart")
        .status()
        .unwrap()
        .success();
    assert!(ret);

    let ret = Command::new("prettier")
        .arg("--write")
        .arg("../js/bindings.js")
        .status()
        .unwrap()
        .success();
    assert!(ret);

    let ret = Command::new("prettier")
        .arg("--write")
        .arg("../js/bindings.d.ts")
        .status()
        .unwrap()
        .success();
    assert!(ret);
}
