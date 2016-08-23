extern crate cretonne;
extern crate cton_reader;
extern crate regex;

use regex::Regex;
use self::cretonne::ir::Ebb;
use self::cton_reader::parser::Parser;
use self::cretonne::ir::function::Function;
use self::cretonne::entity_map::EntityMap;
use self::cretonne::ir::entities::NO_INST;
use self::cretonne::cfg::ControlFlowGraph;
use self::cretonne::dominator_tree::DominatorTree;

/// Construct a dominator tree from specially formatted comments in
/// cton source. Each line with a  jump/branch instruction should
/// have a comment of the format: `dominates(n, ..., N)`, where each `n`
/// is the Ebb number for which this instruction is the immediate dominator.
fn dominator_tree_from_source(func: &Function, function_source: &str) -> DominatorTree {
    let ebb_re = Regex::new("^[ \t]*ebb[0-9]+.*:").unwrap();
    let dom_re = Regex::new("dominates\\(([0-9,]+)\\)").unwrap();
    let inst_re = Regex::new("^[ \t]*[a-zA-Z0-9]+[^{}]*").unwrap();
    let func_re = Regex::new("^[ \t]*function.*").unwrap();

    let ebbs = func.layout.ebbs().collect::<Vec<_>>();
    let mut data = EntityMap::with_capacity(ebbs.len());

    if ebbs.len() < 1 {
        return DominatorTree::from_data(data);
    }

    let mut ebb_offset = 0;
    let mut inst_offset = 0;

    let mut cur_ebb = ebbs[0];
    let mut insts = func.layout.ebb_insts(ebbs[ebb_offset]).collect::<Vec<_>>();

    for line in function_source.lines() {
        if ebb_re.is_match(line) {
            cur_ebb = ebbs[ebb_offset];
            insts = func.layout.ebb_insts(cur_ebb).collect::<Vec<_>>();
            ebb_offset += 1;
            inst_offset = 0;
        } else if inst_re.is_match(line) && !func_re.is_match(line) {
            inst_offset += 1;
        }
        match dom_re.captures(line) {
            Some(caps) => {
                for s in caps.at(1).unwrap().split(",") {
                    let this_ebb = Ebb::with_number(s.parse::<u32>().unwrap()).unwrap();
                    let inst = if inst_offset == 0 {
                        NO_INST
                    } else {
                        insts[inst_offset - 1].clone()
                    };
                    data[this_ebb] = Some((cur_ebb.clone(), inst));
                }
            },
            None => continue,
        };

    }
    DominatorTree::from_data(data)
}

fn test_dominator_tree(function_source: &str) {

    let func = &Parser::parse(function_source).unwrap()[0];
    let src_dtree = dominator_tree_from_source(&func, function_source);

    let cfg = ControlFlowGraph::new(&func);
    let dtree = DominatorTree::new(&cfg);

    for ebb in func.layout.ebbs() {
        assert_eq!(dtree.idom(ebb), src_dtree.idom(ebb));
    }
}

#[test]
fn basic() {
    test_dominator_tree("
        function test(i32) {
            ebb0(v0: i32):    ; dominates(0)
                jump ebb1     ; dominates(1)
            ebb1:
                brz v0, ebb3  ; dominates(3)
                jump ebb2     ; dominates(2)
            ebb2:
                jump ebb3
            ebb3:
                return
        }
    ");
}

#[test]
fn loops() {
    test_dominator_tree("
        function test(i32) {
            ebb0(v0: i32):    ; dominates(0)
                brz v0, ebb1  ; dominates(1,3,4,5)
                jump ebb2     ; dominates(2)
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
    ");
}
