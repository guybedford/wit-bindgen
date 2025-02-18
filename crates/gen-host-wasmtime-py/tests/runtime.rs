use std::fs;
use std::path::Path;
use std::process::Command;
use wit_bindgen_core::Generator;

test_helpers::runtime_tests!("py");

fn execute(name: &str, wasm: &Path, py: &Path, imports: &Path, exports: &Path) {
    let dir = test_helpers::test_directory("runtime", "wasmtime-py", name);
    fs::create_dir_all(&dir.join("imports")).unwrap();
    fs::create_dir_all(&dir.join("exports")).unwrap();

    println!("OUT_DIR = {:?}", dir);
    println!("Generating bindings...");
    // We call `generate_all` with exports from the imports.wit file, and
    // imports from the exports.wit wit file. It's reversed because we're
    // implementing the host side of these APIs.
    let iface = wit_bindgen_core::wit_parser::Interface::parse_file(imports).unwrap();
    let mut files = Default::default();
    wit_bindgen_gen_host_wasmtime_py::Opts::default()
        .build()
        .generate_all(&[], &[iface], &mut files);
    for (file, contents) in files.iter() {
        fs::write(dir.join("imports").join(file), contents).unwrap();
    }
    fs::write(dir.join("imports").join("__init__.py"), "").unwrap();

    let iface = wit_bindgen_core::wit_parser::Interface::parse_file(exports).unwrap();
    let mut files = Default::default();
    wit_bindgen_gen_host_wasmtime_py::Opts::default()
        .build()
        .generate_all(&[iface], &[], &mut files);
    for (file, contents) in files.iter() {
        fs::write(dir.join("exports").join(file), contents).unwrap();
    }
    fs::write(dir.join("exports").join("__init__.py"), "").unwrap();

    println!("Running mypy...");
    exec(
        Command::new("mypy")
            .env("MYPYPATH", &dir)
            .arg(py)
            .arg("--cache-dir")
            .arg(dir.parent().unwrap().join("mypycache").join(name)),
    );

    exec(
        Command::new("python3")
            .env("PYTHONPATH", &dir)
            .arg(py)
            .arg(wasm),
    );
}

fn exec(cmd: &mut Command) {
    println!("{:?}", cmd);
    let output = cmd.output().unwrap();
    if output.status.success() {
        return;
    }
    println!("status: {}", output.status);
    println!(
        "stdout ---\n  {}",
        String::from_utf8_lossy(&output.stdout).replace("\n", "\n  ")
    );
    println!(
        "stderr ---\n  {}",
        String::from_utf8_lossy(&output.stderr).replace("\n", "\n  ")
    );
    panic!("no success");
}
