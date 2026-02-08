use skeplib::ast::{FnDecl, ImportDecl, Program};

#[test]
fn create_empty_program() {
    let program = Program::default();
    assert!(program.imports.is_empty());
    assert!(program.functions.is_empty());
}

#[test]
fn create_program_with_one_import_and_one_function() {
    let program = Program {
        imports: vec![ImportDecl {
            module: "io".to_string(),
        }],
        functions: vec![FnDecl {
            name: "main".to_string(),
        }],
    };

    assert_eq!(program.imports.len(), 1);
    assert_eq!(program.imports[0].module, "io");
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "main");
}
