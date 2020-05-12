//! Compilation backend pipeline: optimized IR to VCode / binemit.

use crate::ir::Function;
use crate::machinst::*;
use crate::settings;
use crate::timing;

use log::debug;
use regalloc::{allocate_registers_with_opts, Algorithm, Options};

/// Compile the given function down to VCode with allocated registers, ready
/// for binary emission.
pub fn compile<B: LowerBackend + MachBackend>(
    f: &Function,
    b: &B,
    abi: Box<dyn ABIBody<I = B::MInst>>,
) -> CodegenResult<VCode<B::MInst>>
where
    B::MInst: ShowWithRRU,
{
    // This lowers the CL IR.
    let mut vcode = Lower::new(f, abi)?.lower(b)?;

    debug!(
        "vcode from lowering: \n{}",
        vcode.show_rru(Some(b.reg_universe()))
    );

    // Perform register allocation.
    let (run_checker, algorithm) = match vcode.flags().regalloc() {
        settings::Regalloc::Backtracking => (false, Algorithm::Backtracking(Default::default())),
        settings::Regalloc::BacktrackingChecked => {
            (true, Algorithm::Backtracking(Default::default()))
        }
        settings::Regalloc::ExperimentalLinearScan => {
            (false, Algorithm::LinearScan(Default::default()))
        }
        settings::Regalloc::ExperimentalLinearScanChecked => {
            (true, Algorithm::LinearScan(Default::default()))
        }
    };

    let result = {
        let _tt = timing::regalloc();
        allocate_registers_with_opts(
            &mut vcode,
            b.reg_universe(),
            Options {
                run_checker,
                algorithm,
            },
        )
        .map_err(|err| {
            debug!(
                "Register allocation error for vcode\n{}\nError: {:?}",
                vcode.show_rru(Some(b.reg_universe())),
                err
            );
            err
        })
        .expect("register allocation")
    };

    // Reorder vcode into final order and copy out final instruction sequence
    // all at once. This also inserts prologues/epilogues.
    vcode.replace_insns_from_regalloc(result);

    vcode.remove_redundant_branches();

    // Do final passes over code to finalize branches.
    vcode.finalize_branches();

    debug!(
        "vcode after regalloc: final version:\n{}",
        vcode.show_rru(Some(b.reg_universe()))
    );

    Ok(vcode)
}
