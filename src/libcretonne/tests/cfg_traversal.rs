extern crate cretonne;
extern crate cton_reader;

use self::cton_reader::parser::Parser;
use self::cretonne::ir::entities::Ebb;
use self::cretonne::cfg::ControlFlowGraph;

fn test_reverse_postorder_traversal(function_source: &str, ebb_order: Vec<u32>) {
    let func = &Parser::parse(function_source).unwrap()[0];
    let cfg = ControlFlowGraph::new(&func);
    let ebbs = ebb_order.iter().map(|n| Ebb::with_number(*n).unwrap())
                               .collect::<Vec<Ebb>>();

    let reverse_postorder_ebbs = cfg.reverse_postorder_ebbs();

    assert_eq!(reverse_postorder_ebbs.len(), ebbs.len());
    for ebb in reverse_postorder_ebbs.keys() {
        assert_eq!(ebb, ebbs[ebbs.len() - reverse_postorder_ebbs[ebb]]);
    }
}

#[test]
fn simple_traversal() {
    test_reverse_postorder_traversal("
        function test(i32) {
            ebb0(v0: i32):
               brz v0, ebb1
               jump ebb2
            ebb1:
                jump ebb3
            ebb2:
                v1 = iconst.i32 1
                v2 = iadd v1, v0
                brz v2, ebb2
                v3 = iadd v1, v2
                brz v3, ebb1
                v4 = iadd v1, v3
                brz v4, ebb4
                jump ebb5
            ebb3:
                trap
            ebb4:
                trap
            ebb5:
                trap
        }
    ", vec![0, 2, 1, 3, 4, 5]);
}

#[test]
fn loops_one() {
    test_reverse_postorder_traversal("
        function test(i32) {
            ebb0(v0: i32):
                jump ebb1
            ebb1:
                brnz v0, ebb3
                jump ebb2
            ebb2:
                jump ebb3
            ebb3:
                return
        }
    ", vec![0, 1, 2, 3]);
}

#[test]
fn loops_two() {
    test_reverse_postorder_traversal("
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
    ", vec![0, 1, 2, 5, 4, 3]);
}

#[test]
fn loops_three() {
    test_reverse_postorder_traversal("
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
                brnz v0, ebb5
                jump ebb6
            ebb5:
                brz v0, ebb4
                trap
            ebb6:
                jump ebb7
            ebb7:
                return
        }
    ", vec![0, 1, 2, 5, 4, 3, 6, 7]);
}

#[test]
fn back_edge_one() {
    test_reverse_postorder_traversal("
        function test(i32) {
            ebb0(v0: i32):
                brz v0, ebb1
                jump ebb2
            ebb1:
                jump ebb3
            ebb2:
                brz v0, ebb0
                jump ebb4
            ebb3:
                brz v0, ebb2
                brnz v0, ebb0
                return
            ebb4:
                trap
        }
    ", vec![0, 1, 3, 2, 4]);
}
