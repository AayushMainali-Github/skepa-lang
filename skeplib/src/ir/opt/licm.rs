use crate::ir::IrProgram;

pub fn run(program: &mut IrProgram) -> bool {
    let mut changed = false;

    for func in &mut program.functions {
        let ids: Vec<_> = func.blocks.iter().map(|block| block.id).collect();
        for header_id in ids {
            let Some(header_idx) = func.blocks.iter().position(|block| block.id == header_id)
            else {
                continue;
            };
            let header_name = func.blocks[header_idx].name.clone();
            if header_name != "while_cond" && header_name != "for_cond" {
                continue;
            }
            let Some(loop_blocks) = related_loop_blocks(func, header_id) else {
                continue;
            };
            let Some(preheader_id) = find_preheader(func, header_id, &loop_blocks) else {
                continue;
            };
            let Some(preheader_idx) = func
                .blocks
                .iter()
                .position(|block| block.id == preheader_id)
            else {
                continue;
            };

            for block_id in loop_blocks {
                let Some(loop_idx) = func.blocks.iter().position(|block| block.id == block_id)
                else {
                    continue;
                };
                let mut split_at = 0usize;
                for instr in &func.blocks[loop_idx].instrs {
                    if matches!(instr, crate::ir::Instr::Const { .. }) {
                        split_at += 1;
                    } else {
                        break;
                    }
                }
                if split_at == 0 {
                    continue;
                }
                let hoisted = func.blocks[loop_idx]
                    .instrs
                    .drain(..split_at)
                    .collect::<Vec<_>>();
                func.blocks[preheader_idx].instrs.extend(hoisted);
                changed = true;
            }
        }
    }

    changed
}

fn find_preheader(
    func: &crate::ir::IrFunction,
    header: crate::ir::BlockId,
    loop_blocks: &[crate::ir::BlockId],
) -> Option<crate::ir::BlockId> {
    let preds = predecessors(func, header);
    if preds.len() != 2 {
        return None;
    }
    preds.into_iter().find(|pred| !loop_blocks.contains(pred))
}

fn predecessors(
    func: &crate::ir::IrFunction,
    target: crate::ir::BlockId,
) -> Vec<crate::ir::BlockId> {
    let mut out = Vec::new();
    for block in &func.blocks {
        match &block.terminator {
            crate::ir::Terminator::Jump(next) if *next == target => out.push(block.id),
            crate::ir::Terminator::Branch(branch)
                if branch.then_block == target || branch.else_block == target =>
            {
                out.push(block.id);
            }
            _ => {}
        }
    }
    out
}

fn related_loop_blocks(
    func: &crate::ir::IrFunction,
    header: crate::ir::BlockId,
) -> Option<Vec<crate::ir::BlockId>> {
    let header_block = func.blocks.iter().find(|block| block.id == header)?;
    let body = match &header_block.terminator {
        crate::ir::Terminator::Branch(branch) => branch.then_block,
        _ => return None,
    };
    let mut blocks = vec![header, body];
    if header_block.name == "for_cond"
        && let Some(crate::ir::Terminator::Jump(step)) = func
            .blocks
            .iter()
            .find(|block| block.id == body)
            .map(|block| &block.terminator)
    {
        blocks.push(*step);
    }
    Some(blocks)
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::ir::{
        BasicBlock, BlockId, BranchTerminator, ConstValue, FunctionId, Instr, IrFunction,
        IrProgram, IrType, TempId, Terminator,
    };

    #[test]
    fn repeated_loop_names_do_not_select_the_first_body() {
        let mut entry = BasicBlock::new(BlockId(0), "entry");
        entry.terminator = Terminator::Jump(BlockId(1));

        let mut first_cond = BasicBlock::new(BlockId(1), "while_cond");
        first_cond.terminator = Terminator::Branch(BranchTerminator {
            cond: crate::ir::Operand::Const(ConstValue::Bool(true)),
            then_block: BlockId(2),
            else_block: BlockId(3),
        });

        let mut first_body = BasicBlock::new(BlockId(2), "while_body");
        first_body.instrs.push(Instr::Const {
            dst: TempId(0),
            ty: IrType::Int,
            value: ConstValue::Int(10),
        });
        first_body.terminator = Terminator::Jump(BlockId(1));

        let mut first_exit = BasicBlock::new(BlockId(3), "while_exit");
        first_exit.terminator = Terminator::Jump(BlockId(4));

        let mut second_cond = BasicBlock::new(BlockId(4), "while_cond");
        second_cond.terminator = Terminator::Branch(BranchTerminator {
            cond: crate::ir::Operand::Const(ConstValue::Bool(false)),
            then_block: BlockId(5),
            else_block: BlockId(6),
        });

        let mut second_body = BasicBlock::new(BlockId(5), "while_body");
        second_body.instrs.push(Instr::Const {
            dst: TempId(1),
            ty: IrType::Int,
            value: ConstValue::Int(20),
        });
        second_body.terminator = Terminator::Jump(BlockId(4));

        let mut second_exit = BasicBlock::new(BlockId(6), "while_exit");
        second_exit.terminator = Terminator::Return(None);

        let function = IrFunction {
            id: FunctionId(0),
            name: "main".into(),
            params: Vec::new(),
            locals: Vec::new(),
            temps: vec![
                crate::ir::IrTemp {
                    id: TempId(0),
                    ty: IrType::Int,
                },
                crate::ir::IrTemp {
                    id: TempId(1),
                    ty: IrType::Int,
                },
            ],
            ret_ty: IrType::Void,
            entry: BlockId(0),
            blocks: vec![
                entry,
                first_cond,
                first_body,
                first_exit,
                second_cond,
                second_body,
                second_exit,
            ],
        };
        let mut program = IrProgram {
            structs: Vec::new(),
            globals: Vec::new(),
            functions: vec![function],
            module_init: None,
        };

        assert!(run(&mut program));
        let blocks = &program.functions[0].blocks;
        assert!(blocks[0].instrs.iter().any(|instr| matches!(
            instr,
            Instr::Const {
                value: ConstValue::Int(10),
                ..
            }
        )));
        assert!(blocks[3].instrs.iter().any(|instr| matches!(
            instr,
            Instr::Const {
                value: ConstValue::Int(20),
                ..
            }
        )));
    }
}
