use skeplib::bytecode::compile_project_entry;
use skeplib::resolver::resolve_project;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn make_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("skepa_perf_{label}_{nanos}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn resolver_and_compiler_handle_many_modules_and_deep_folder_import() {
    let root = make_temp_dir("many_modules_deep_folder");
    fs::create_dir_all(root.join("pkg").join("d1").join("d2").join("d3").join("d4"))
        .expect("create nested folders");

    let module_count = 60usize;
    for i in 0..module_count {
        let name = format!("m{i}");
        let next = format!("m{}", (i + 1) % module_count);
        let src = if i + 1 < module_count {
            format!(
                "from {next} import f{};\nfn f{i}() -> Int {{ return f{}() + 1; }}\nexport {{ f{i} }};\n",
                i + 1,
                i + 1,
            )
        } else {
            "fn f59() -> Int { return 1; }\nexport { f59 };\n".to_string()
        };
        fs::write(root.join(format!("{name}.sk")), src).expect("write module");
    }

    fs::write(
        root.join("pkg").join("d1").join("d2").join("d3").join("d4").join("leaf.sk"),
        "fn leaf() -> Int { return 3; }\nexport { leaf };\n",
    )
    .expect("write leaf");

    fs::write(
        root.join("main.sk"),
        "from m0 import f0;\nimport pkg;\nfn main() -> Int { return f0() + pkg.d1.d2.d3.d4.leaf(); }\n",
    )
    .expect("write main");

    let graph = resolve_project(&root.join("main.sk")).expect("resolve large project");
    assert!(graph.modules.len() >= module_count + 2);
    let module = compile_project_entry(&root.join("main.sk")).expect("compile large project");
    assert!(!module.functions.is_empty());
    let _ = fs::remove_dir_all(root);
}
