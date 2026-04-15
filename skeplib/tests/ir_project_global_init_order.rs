mod common;

use skepart::RtValue;
use skeplib::ir::{IrInterpreter, lowering};

#[test]
fn project_global_init_runs_dependencies_before_dependents() {
    let project = common::TempProject::new("project_global_init_dependency_order");
    project.file(
        "b.sk",
        r#"
let seed: Int = 40;
export { seed };
"#,
    );
    project.file(
        "a.sk",
        r#"
from b import seed;
let answer: Int = seed + 2;
export { answer };
"#,
    );
    let entry = project.file(
        "main.sk",
        r#"
from a import answer;
fn main() -> Int { return answer; }
"#,
    );

    let program = lowering::compile_project_entry(&entry).expect("project lowering should succeed");
    let value = IrInterpreter::new(&program)
        .run_main()
        .expect("IR interpreter should run");
    assert_eq!(value, RtValue::Int(42));
}
