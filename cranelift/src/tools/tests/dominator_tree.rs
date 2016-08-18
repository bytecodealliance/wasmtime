extern crate cretonne;
extern crate cton_reader;

use self::cretonne::ir::Ebb;
use self::cton_reader::parser::Parser;
use self::cretonne::ir::entities::NO_INST;
use self::cretonne::cfg::ControlFlowGraph;
use self::cretonne::ir::instructions::BranchInfo;
use self::cretonne::dominator_tree::DominatorTree;

fn test_dominator_tree(function_source: &str, idoms: Vec<u32>) {
    let func = &Parser::parse(function_source).unwrap()[0];
    let cfg = ControlFlowGraph::new(&func);
    let dtree = DominatorTree::new(&cfg);
    assert_eq!(dtree.ebbs().collect::<Vec<_>>().len(), idoms.len());
    for (i, j) in idoms.iter().enumerate() {
        let ebb = Ebb::with_number(i.clone() as u32).unwrap();
        let idom_ebb = Ebb::with_number(*j).unwrap();
        let mut idom_inst = NO_INST;

        // Find the first branch/jump instruction which points to the idom_ebb
        // and use it to denote our idom basic block.
        for inst in func.layout.ebb_insts(idom_ebb) {
           match func.dfg[inst].analyze_branch() {
                BranchInfo::SingleDest(dest, _) => {
                    if dest == ebb {
                        idom_inst = inst;
                        break;
                    }
                }
                BranchInfo::Table(jt) => {
                    for (_, dest) in func.jump_tables[jt].entries() {
                        if dest == ebb {
                            idom_inst = inst;
                            break;
                        }
                    }
                    // We already found our inst!
                    if idom_inst != NO_INST {
                        break;
                    }
                }
                BranchInfo::NotABranch => {}
            }
        }
        assert_eq!(dtree.idom(ebb).unwrap(), (idom_ebb, idom_inst));
    }
}

#[test]
fn basic() {
    test_dominator_tree("
        function test(i32) {
            ebb0(v0: i32):
                jump ebb1
            ebb1:
                brz v0, ebb3
                jump ebb2
            ebb2:
                jump ebb3
            ebb3:
                return
        }
    ", vec![0, 0, 1, 1]);
}

#[test]
fn loops() {
    test_dominator_tree("
        function test(i32) {
            ebb0(v0: i32):
                brz v0, ebb1
                jump ebb2
            ebb1:
                jump ebb3
            ebb2:
                brz v0, ebb4
                jump ebb5
            ebb3:
                jump ebb4
            ebb4:
                brz v0, ebb3
                jump ebb5
            ebb5:
                brz v0, ebb4
                return
        }
    ", vec![0, 0, 0, 0, 0, 0]);
}
