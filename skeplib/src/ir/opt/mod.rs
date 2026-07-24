mod cfg_simplify;
mod const_fold;
mod copy_prop;
mod dce;
mod dead_store;
mod inlining;
mod licm;
mod loop_simplify;
mod resolve_calls;
mod strength_reduce;

use crate::ir::IrProgram;

#[derive(Debug, Clone, Copy)]
struct OptimizeOptions {
    inline: bool,
}

pub fn optimize_program(program: &mut IrProgram) {
    optimize_program_with(program, OptimizeOptions { inline: true });
}

/// Optimize IR while keeping call boundaries intact.
///
/// Partitioned multi-module native builds cache objects per module. Cross-module
/// inlining embeds callee bodies into callers and couples partition fingerprints,
/// so incremental rebuilds would invalidate unrelated modules.
pub fn optimize_program_for_partitions(program: &mut IrProgram) {
    optimize_program_with(program, OptimizeOptions { inline: false });
}

fn optimize_program_with(program: &mut IrProgram, options: OptimizeOptions) {
    loop {
        let mut changed = false;
        changed |= const_fold::run(program);
        changed |= copy_prop::run(program);
        changed |= dce::run(program);
        changed |= cfg_simplify::run(program);
        if options.inline {
            changed |= inlining::run(program);
        }
        changed |= dead_store::run(program);
        changed |= loop_simplify::run(program);
        changed |= licm::run(program);
        changed |= strength_reduce::run(program);
        changed |= resolve_calls::run(program);
        if !changed {
            break;
        }
    }
}
