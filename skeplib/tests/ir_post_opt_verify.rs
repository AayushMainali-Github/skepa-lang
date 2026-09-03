use skeplib::ir::{self, FunctionId, IrFunction, IrProgram, IrType};

#[test]
fn checked_optimizer_rejects_invalid_ir_after_optimization() {
    let mut program = IrProgram::new();
    program.functions.push(IrFunction {
        id: FunctionId(0),
        name: "invalid".to_string(),
        params: Vec::new(),
        locals: Vec::new(),
        temps: Vec::new(),
        ret_ty: IrType::Void,
        entry: ir::BlockId(0),
        blocks: Vec::new(),
    });

    let error = ir::opt::optimize_program_checked(&mut program)
        .expect_err("post-optimization verification should reject missing blocks");
    assert!(matches!(error, ir::IrVerifyError::MissingEntryBlock { .. }));
}

#[test]
fn checked_partition_optimizer_rejects_invalid_ir_after_optimization() {
    let mut program = IrProgram::new();
    program.functions.push(IrFunction {
        id: FunctionId(0),
        name: "invalid".to_string(),
        params: Vec::new(),
        locals: Vec::new(),
        temps: Vec::new(),
        ret_ty: IrType::Void,
        entry: ir::BlockId(0),
        blocks: Vec::new(),
    });

    let error = ir::opt::optimize_program_for_partitions_checked(&mut program)
        .expect_err("checked partition optimization should reject missing blocks");
    assert!(matches!(error, ir::IrVerifyError::MissingEntryBlock { .. }));
}
