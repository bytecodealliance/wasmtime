//! Elaboration phase: lowers EGraph back to sequences of operations
//! in CFG nodes.

use super::domtree::DomTreeWithChildren;
use super::extract::Extractor;
use super::node::Node;
use crate::dominator_tree::DominatorTree;
use crate::ir::{Block, Function, SourceLoc, Type, Value, ValueList};
use crate::scoped_hash_map::ScopedHashMap;
use cranelift_egraph::{EGraph, Id, Language};
use smallvec::SmallVec;

pub(crate) struct Elaborator<'a> {
    func: &'a mut Function,
    domtree: &'a DominatorTree,
    egraph: &'a EGraph<Node<'a>>,
    extractor: &'a Extractor,
    id_to_value: ScopedHashMap<Id, IdValue>,
    cur_block: Option<Block>,
}

#[derive(Clone, Debug)]
enum IdValue {
    /// A single value.
    Value(Value),
    /// Multiple results; indices in `node_args`.
    Values(ValueList),
}

impl<'a> Elaborator<'a> {
    pub(crate) fn new(
        func: &'a mut Function,
        domtree: &'a DominatorTree,
        egraph: &'a EGraph<Node>,
        extractor: &'a Extractor,
    ) -> Self {
        Self {
            func,
            domtree,
            egraph,
            extractor,
            id_to_value: ScopedHashMap::new(),
            cur_block: None,
        }
    }

    fn start_block(&mut self, block: Block, block_params: &[(Id, Type)]) {
        self.cur_block = Some(block);
        for &(id, ty) in block_params {
            let val = self.func.dfg.append_block_param(block, ty);
            self.id_to_value.insert_if_absent(id, IdValue::Value(val));
        }
    }

    // TODO: LICM: when adding node, append to first level of loop
    // nest at which we have input args. Track loop nest as we do
    // domtree traversal?

    fn add_node(&mut self, node: &Node, args: &[Value]) -> ValueList {
        let (instdata, result_tys) = match node {
            Node::Pure { op, types, .. } | Node::Inst { op, types, .. } => {
                (op.with_args(args, &mut self.func.dfg.value_lists), types)
            }
            _ => panic!("Cannot `add_node()` on block param or projection"),
        };
        let srcloc = match node {
            Node::Inst { srcloc, .. } => *srcloc,
            _ => SourceLoc::default(),
        };
        let inst = self.func.dfg.make_inst(instdata);
        self.func.srclocs[inst] = srcloc;
        for &ty in result_tys.iter() {
            self.func.dfg.append_result(inst, ty);
        }
        self.func.layout.append_inst(inst, self.cur_block.unwrap());
        self.func.dfg.inst_results_list(inst)
    }

    fn elaborate_eclass_use(&mut self, id: Id) -> IdValue {
        if let Some(val) = self.id_to_value.get(&id) {
            return val.clone();
        }

        let node = self
            .extractor
            .get_node(self.egraph, id)
            .expect("Should have extracted node for eclass that is used");

        // Is the node a block param? We should never get here if so
        // (they are inserted when first visiting the block).
        if matches!(node, Node::Param { .. }) {
            unreachable!("Param nodes should already be inserted");
        }

        // Is the node a result projection? If so, at this point we
        // have everything we need; no need to allocate a new Value
        // for the result.
        if let Node::Result { value, result, .. } = node {
            let value = self.elaborate_eclass_use(*value);
            let range = match value {
                IdValue::Values(range) => range,
                IdValue::Value(_) => {
                    unreachable!("Projection nodes should not be used on single results");
                }
            };
            let values = range.as_slice(&self.func.dfg.value_lists);
            let value = IdValue::Value(values[*result]);
            self.id_to_value.insert_if_absent(id, value.clone());
            return value;
        }

        // We're going to need to emit this operator. First, elaborate
        // all args, recursively.
        let args: SmallVec<[Value; 8]> = node
            .children()
            .iter()
            .map(|&id| self.elaborate_eclass_use(id))
            .map(|idvalue| match idvalue {
                IdValue::Value(value) => value,
                IdValue::Values(..) => panic!("enode depends directly on multi-value result"),
            })
            .collect();

        // This is an actual operation; emit the node in sequence now.
        let results = self.add_node(node, &args[..]);
        let results_slice = results.as_slice(&self.func.dfg.value_lists);

        // Build the result and memoize in the id-to-value map.
        let result = if results_slice.len() == 1 {
            IdValue::Value(results_slice[0])
        } else {
            IdValue::Values(results)
        };

        self.id_to_value.insert_if_absent(id, result.clone());
        result
    }

    fn elaborate_block<'b, PF: Fn(Block) -> &'b [(Id, Type)], RF: Fn(Block) -> &'b [Id]>(
        &mut self,
        block: Block,
        block_params_fn: &PF,
        block_roots_fn: &RF,
        domtree: &DomTreeWithChildren,
    ) {
        self.id_to_value.increment_depth();

        let blockparam_ids_tys = (block_params_fn)(block);
        self.start_block(block, blockparam_ids_tys);
        for &id in (block_roots_fn)(block) {
            self.elaborate_eclass_use(id);
        }

        for child in domtree.children(block) {
            self.elaborate_block(child, block_params_fn, block_roots_fn, domtree);
        }

        self.id_to_value.decrement_depth();
    }

    fn clear_func_body(&mut self) {
        // Clear all instructions and args/results from the DFG. We
        // rebuild them entirely during elaboration. (TODO: reuse the
        // existing inst for the *first* copy of a given node.)
        self.func.dfg.clear_insts();
        // Clear the instructions in every block, but leave the list
        // of blocks and their layout unmodified.
        self.func.layout.clear_insts();
        self.func.srclocs.clear();
    }

    pub(crate) fn elaborate<'b, PF: Fn(Block) -> &'b [(Id, Type)], RF: Fn(Block) -> &'b [Id]>(
        &mut self,
        block_params_fn: PF,
        block_roots_fn: RF,
    ) {
        let domtree = DomTreeWithChildren::new(self.func, self.domtree);
        let root = domtree.root();
        self.clear_func_body();
        self.elaborate_block(root, &block_params_fn, &block_roots_fn, &domtree);
    }
}
