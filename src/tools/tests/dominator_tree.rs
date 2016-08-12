extern crate cretonne;
extern crate cton_reader;

use self::cton_reader::parser::Parser;
use self::cretonne::ir::Ebb;
use self::cretonne::cfg::ControlFlowGraph;
use self::cretonne::dominator_tree::DominatorTree;

fn test_dominator_tree(function_source: &str, idoms: Vec<u32>) {
    let func = &Parser::parse(function_source).unwrap()[0];
    let cfg = ControlFlowGraph::new(&func);
    let dtree = DominatorTree::new(&cfg);
    assert_eq!(dtree.ebbs().collect::<Vec<_>>().len(), idoms.len());
    for (i, j) in idoms.iter().enumerate() {
        let ebb = Ebb::with_number(i.clone() as u32);
        let idom = Ebb::with_number(*j);
        assert_eq!(dtree.idom(ebb.unwrap()), idom);
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
