use cranelift_codegen::ir::{Block, Function, Inst, InstructionData, Signature};
use serde_derive::{Deserialize, Serialize};

/// Serializable version of the original Cranelift IR
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SerInstData {
    Unary {
        opcode: String,
        arg: String,
    },
    UnaryImm {
        opcode: String,
        imm: String,
    },
    UnaryIeee32 {
        opcode: String,
        imm: String,
    },
    UnaryIeee64 {
        opcode: String,
        imm: String,
    },
    UnaryBool {
        opcode: String,
        imm: bool,
    },
    UnaryGlobalValue {
        opcode: String,
        global_value: String,
    },
    Binary {
        opcode: String,
        args: [String; 2],
    },
    BinaryImm8 {
        opcode: String,
        arg: String,
        imm: String,
    },
    BinaryImm64 {
        opcode: String,
        arg: String,
        imm: String,
    },
    Ternary {
        opcode: String,
        args: [String; 3],
    },
    TernaryImm8 {
        opcode: String,
        args: [String; 2],
        imm: String,
    },
    MultiAry {
        opcode: String,
        args: Vec<String>,
    },
    NullAry {
        opcode: String,
    },
    Shuffle {
        opcode: String,
        args: [String; 2],
        mask: String,
    },
    IntCompare {
        opcode: String,
        args: [String; 2],
        cond: String,
    },
    IntCompareImm {
        opcode: String,
        arg: String,
        cond: String,
        imm: String,
    },
    IntCond {
        opcode: String,
        arg: String,
        cond: String,
    },
    FloatCompare {
        opcode: String,
        args: [String; 2],
        cond: String,
    },
    FloatCond {
        opcode: String,
        arg: String,
        cond: String,
    },
    IntSelect {
        opcode: String,
        args: [String; 3],
        cond: String,
    },
    Jump {
        opcode: String,
        args: Vec<String>,
        destination: String,
    },
    Branch {
        opcode: String,
        args: Vec<String>,
        destination: String,
    },
    BranchInt {
        opcode: String,
        args: Vec<String>,
        cond: String,
        destination: String,
    },
    BranchFloat {
        opcode: String,
        args: Vec<String>,
        cond: String,
        destination: String,
    },
    BranchIcmp {
        opcode: String,
        args: Vec<String>,
        cond: String,
        destination: String,
    },
    BranchTable {
        opcode: String,
        arg: String,
        destination: String,
        table: String,
    },
    BranchTableEntry {
        opcode: String,
        args: [String; 2],
        imm: String,
        table: String,
    },
    BranchTableBase {
        opcode: String,
        table: String,
    },
    IndirectJump {
        opcode: String,
        arg: String,
        table: String,
    },
    Call {
        opcode: String,
        args: Vec<String>,
        func_ref: String,
    },
    CallIndirect {
        opcode: String,
        args: Vec<String>,
        sig_ref: String,
    },
    FuncAddr {
        opcode: String,
        func_ref: String,
    },
    Load {
        opcode: String,
        arg: String,
        flags: String,
        offset: String,
    },
    LoadComplex {
        opcode: String,
        args: Vec<String>,
        flags: String,
        offset: String,
    },
    Store {
        opcode: String,
        args: [String; 2],
        flags: String,
        offset: String,
    },
    StoreComplex {
        opcode: String,
        args: Vec<String>,
        flags: String,
        offset: String,
    },
    StackLoad {
        opcode: String,
        stack_slot: String,
        offset: String,
    },
    StackStore {
        opcode: String,
        arg: String,
        stack_slot: String,
        offset: String,
    },
    HeapAddr {
        opcode: String,
        arg: String,
        heap: String,
        imm: String,
    },
    TableAddr {
        opcode: String,
        arg: String,
        table: String,
        offset: String,
    },
    RegMove {
        opcode: String,
        arg: String,
        src: String,
        dst: String,
    },
    CopySpecial {
        opcode: String,
        src: String,
        dst: String,
    },
    CopyToSsa {
        opcode: String,
        src: String,
    },
    RegSpill {
        opcode: String,
        arg: String,
        src: String,
        dst: String,
    },
    RegFill {
        opcode: String,
        arg: String,
        src: String,
        dst: String,
    },
    Trap {
        opcode: String,
        code: String,
    },
    CondTrap {
        opcode: String,
        arg: String,
        code: String,
    },
    IntCondTrap {
        opcode: String,
        arg: String,
        cond: String,
        code: String,
    },
    FloatCondTrap {
        opcode: String,
        arg: String,
        cond: String,
        code: String,
    },
}

/// Convert Cranelift IR instructions to JSON format.
pub fn get_inst_data(inst_index: Inst, func: &Function) -> SerInstData {
    let inst = &func.dfg[inst_index];
    match *inst {
        InstructionData::Unary { opcode, arg } => SerInstData::Unary {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
        },
        InstructionData::UnaryImm { opcode, imm } => SerInstData::UnaryImm {
            opcode: opcode.to_string(),
            imm: imm.to_string(),
        },
        InstructionData::UnaryIeee32 { opcode, imm } => SerInstData::UnaryIeee32 {
            opcode: opcode.to_string(),
            imm: imm.to_string(),
        },
        InstructionData::UnaryIeee64 { opcode, imm } => SerInstData::UnaryIeee64 {
            opcode: opcode.to_string(),
            imm: imm.to_string(),
        },
        InstructionData::UnaryBool { opcode, imm } => SerInstData::UnaryBool {
            opcode: opcode.to_string(),
            imm,
        },
        InstructionData::UnaryGlobalValue {
            opcode,
            global_value,
        } => SerInstData::UnaryGlobalValue {
            opcode: opcode.to_string(),
            global_value: global_value.to_string(),
        },
        InstructionData::Binary { opcode, args } => {
            let hold_args = [args[0].to_string(), args[1].to_string()];
            SerInstData::Binary {
                opcode: opcode.to_string(),
                args: hold_args,
            }
        }
        InstructionData::BinaryImm8 { opcode, arg, imm } => SerInstData::BinaryImm8 {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            imm: imm.to_string(),
        },
        InstructionData::BinaryImm64 { opcode, arg, imm } => SerInstData::BinaryImm64 {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            imm: imm.to_string(),
        },
        InstructionData::Ternary { opcode, args } => {
            let hold_args = [
                args[0].to_string(),
                args[1].to_string(),
                args[2].to_string(),
            ];
            SerInstData::Ternary {
                opcode: opcode.to_string(),
                args: hold_args,
            }
        }
        InstructionData::MultiAry { opcode, ref args } => {
            let mut hold_args = Vec::new();
            let args_iter = args.as_slice(&func.dfg.value_lists);
            for arg in args_iter {
                hold_args.push(arg.to_string());
            }

            SerInstData::MultiAry {
                opcode: opcode.to_string(),
                args: hold_args,
            }
        }
        InstructionData::NullAry { opcode } => SerInstData::NullAry {
            opcode: opcode.to_string(),
        },
        InstructionData::TernaryImm8 { opcode, args, imm } => {
            let hold_args = [args[0].to_string(), args[1].to_string()];
            SerInstData::TernaryImm8 {
                opcode: opcode.to_string(),
                args: hold_args,
                imm: imm.to_string(),
            }
        }
        InstructionData::UnaryConst {
            opcode,
            constant_handle,
        } => {
            let constant = func.dfg.constants.get(constant_handle);
            SerInstData::UnaryImm {
                opcode: opcode.to_string(),
                imm: format!("{:?}", constant),
            }
        }
        InstructionData::Shuffle { opcode, args, mask } => {
            let mask = func
                .dfg
                .immediates
                .get(mask)
                .expect("Expected shuffle mask to already be inserted in immediate mapping");
            SerInstData::Shuffle {
                opcode: opcode.to_string(),
                args: [args[0].to_string(), args[1].to_string()],
                mask: format!("{:?}", mask),
            }
        }
        InstructionData::IntCompare { opcode, args, cond } => {
            let hold_args = [args[0].to_string(), args[1].to_string()];
            SerInstData::IntCompare {
                opcode: opcode.to_string(),
                args: hold_args,
                cond: cond.to_string(),
            }
        }
        InstructionData::IntCompareImm {
            opcode,
            arg,
            cond,
            imm,
        } => SerInstData::IntCompareImm {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            cond: cond.to_string(),
            imm: imm.to_string(),
        },
        InstructionData::IntCond { opcode, arg, cond } => SerInstData::IntCond {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            cond: cond.to_string(),
        },
        InstructionData::FloatCompare { opcode, args, cond } => {
            let hold_args = [args[0].to_string(), args[1].to_string()];
            SerInstData::FloatCompare {
                opcode: opcode.to_string(),
                args: hold_args,
                cond: cond.to_string(),
            }
        }
        InstructionData::FloatCond { opcode, arg, cond } => SerInstData::FloatCond {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            cond: cond.to_string(),
        },
        InstructionData::IntSelect { opcode, args, cond } => {
            let hold_args = [
                args[0].to_string(),
                args[1].to_string(),
                args[2].to_string(),
            ];
            SerInstData::IntSelect {
                opcode: opcode.to_string(),
                args: hold_args,
                cond: cond.to_string(),
            }
        }
        InstructionData::Jump {
            opcode,
            ref args,
            destination,
        } => {
            let mut hold_args = Vec::new();
            let args_iter = args.as_slice(&func.dfg.value_lists);
            for arg in args_iter {
                hold_args.push(arg.to_string());
            }
            SerInstData::Jump {
                opcode: opcode.to_string(),
                args: hold_args,
                destination: destination.to_string(),
            }
        }
        InstructionData::Branch {
            opcode,
            ref args,
            destination,
        } => {
            let mut hold_args = Vec::new();
            let args_iter = args.as_slice(&func.dfg.value_lists);
            for arg in args_iter {
                hold_args.push(arg.to_string());
            }
            SerInstData::Branch {
                opcode: opcode.to_string(),
                args: hold_args,
                destination: destination.to_string(),
            }
        }
        InstructionData::BranchInt {
            opcode,
            ref args,
            cond,
            destination,
        } => {
            let mut hold_args = Vec::new();
            let args_iter = args.as_slice(&func.dfg.value_lists);
            for arg in args_iter {
                hold_args.push(arg.to_string());
            }
            SerInstData::BranchInt {
                opcode: opcode.to_string(),
                args: hold_args,
                cond: cond.to_string(),
                destination: destination.to_string(),
            }
        }
        InstructionData::BranchFloat {
            opcode,
            ref args,
            cond,
            destination,
        } => {
            let mut hold_args = Vec::new();
            let args_iter = args.as_slice(&func.dfg.value_lists);
            for arg in args_iter {
                hold_args.push(arg.to_string());
            }
            SerInstData::BranchFloat {
                opcode: opcode.to_string(),
                args: hold_args,
                cond: cond.to_string(),
                destination: destination.to_string(),
            }
        }
        InstructionData::BranchIcmp {
            opcode,
            ref args,
            cond,
            destination,
        } => {
            let mut hold_args = Vec::new();
            let args_iter = args.as_slice(&func.dfg.value_lists);
            for arg in args_iter {
                hold_args.push(arg.to_string());
            }
            SerInstData::BranchIcmp {
                opcode: opcode.to_string(),
                args: hold_args,
                cond: cond.to_string(),
                destination: destination.to_string(),
            }
        }
        InstructionData::BranchTable {
            opcode,
            arg,
            destination,
            table,
        } => SerInstData::BranchTable {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            destination: destination.to_string(),
            table: table.to_string(),
        },
        InstructionData::BranchTableBase { opcode, table } => SerInstData::BranchTableBase {
            opcode: opcode.to_string(),
            table: table.to_string(),
        },
        InstructionData::BranchTableEntry {
            opcode,
            args,
            imm,
            table,
        } => {
            let hold_args = [args[0].to_string(), args[1].to_string()];
            SerInstData::BranchTableEntry {
                opcode: opcode.to_string(),
                args: hold_args,
                imm: imm.to_string(),
                table: table.to_string(),
            }
        }
        InstructionData::IndirectJump { opcode, arg, table } => SerInstData::IndirectJump {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            table: table.to_string(),
        },
        InstructionData::Call {
            opcode,
            ref args,
            func_ref,
        } => {
            let mut hold_args = Vec::new();
            let args_iter = args.as_slice(&func.dfg.value_lists);
            for arg in args_iter {
                hold_args.push(arg.to_string());
            }
            SerInstData::Call {
                opcode: opcode.to_string(),
                args: hold_args,
                func_ref: func_ref.to_string(),
            }
        }
        InstructionData::CallIndirect {
            opcode,
            ref args,
            sig_ref,
        } => {
            let mut hold_args = Vec::new();
            let args_iter = args.as_slice(&func.dfg.value_lists);
            for arg in args_iter {
                hold_args.push(arg.to_string());
            }
            SerInstData::CallIndirect {
                opcode: opcode.to_string(),
                args: hold_args,
                sig_ref: sig_ref.to_string(),
            }
        }
        InstructionData::FuncAddr { opcode, func_ref } => SerInstData::FuncAddr {
            opcode: opcode.to_string(),
            func_ref: func_ref.to_string(),
        },
        InstructionData::Load {
            opcode,
            arg,
            flags,
            offset,
        } => SerInstData::Load {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            flags: flags.to_string(),
            offset: offset.to_string(),
        },
        InstructionData::LoadComplex {
            opcode,
            ref args,
            flags,
            offset,
        } => {
            let mut hold_args = Vec::new();
            let args_iter = args.as_slice(&func.dfg.value_lists);
            for arg in args_iter {
                hold_args.push(arg.to_string());
            }
            SerInstData::LoadComplex {
                opcode: opcode.to_string(),
                args: hold_args,
                flags: flags.to_string(),
                offset: offset.to_string(),
            }
        }
        InstructionData::Store {
            opcode,
            args,
            flags,
            offset,
        } => {
            let hold_args = [args[0].to_string(), args[1].to_string()];
            SerInstData::Store {
                opcode: opcode.to_string(),
                args: hold_args,
                flags: flags.to_string(),
                offset: offset.to_string(),
            }
        }
        InstructionData::StoreComplex {
            opcode,
            ref args,
            flags,
            offset,
        } => {
            let mut hold_args = Vec::new();
            let args_iter = args.as_slice(&func.dfg.value_lists);
            for arg in args_iter {
                hold_args.push(arg.to_string());
            }
            SerInstData::StoreComplex {
                opcode: opcode.to_string(),
                args: hold_args,
                flags: flags.to_string(),
                offset: offset.to_string(),
            }
        }
        InstructionData::StackLoad {
            opcode,
            stack_slot,
            offset,
        } => SerInstData::StackLoad {
            opcode: opcode.to_string(),
            stack_slot: stack_slot.to_string(),
            offset: offset.to_string(),
        },
        InstructionData::StackStore {
            opcode,
            arg,
            stack_slot,
            offset,
        } => SerInstData::StackStore {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            stack_slot: stack_slot.to_string(),
            offset: offset.to_string(),
        },
        InstructionData::HeapAddr {
            opcode,
            arg,
            heap,
            imm,
        } => SerInstData::HeapAddr {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            heap: heap.to_string(),
            imm: imm.to_string(),
        },
        InstructionData::TableAddr {
            opcode,
            arg,
            table,
            offset,
        } => SerInstData::TableAddr {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            table: table.to_string(),
            offset: offset.to_string(),
        },
        InstructionData::RegMove {
            opcode,
            arg,
            src,
            dst,
        } => SerInstData::RegMove {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            src: src.to_string(),
            dst: dst.to_string(),
        },
        InstructionData::CopySpecial { opcode, src, dst } => SerInstData::CopySpecial {
            opcode: opcode.to_string(),
            src: src.to_string(),
            dst: dst.to_string(),
        },
        InstructionData::CopyToSsa { opcode, src } => SerInstData::CopyToSsa {
            opcode: opcode.to_string(),
            src: src.to_string(),
        },
        InstructionData::RegSpill {
            opcode,
            arg,
            src,
            dst,
        } => SerInstData::RegSpill {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            src: src.to_string(),
            dst: dst.to_string(),
        },
        InstructionData::RegFill {
            opcode,
            arg,
            src,
            dst,
        } => SerInstData::RegFill {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            src: src.to_string(),
            dst: dst.to_string(),
        },
        InstructionData::Trap { opcode, code } => SerInstData::Trap {
            opcode: opcode.to_string(),
            code: code.to_string(),
        },
        InstructionData::CondTrap { opcode, arg, code } => SerInstData::CondTrap {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            code: code.to_string(),
        },
        InstructionData::IntCondTrap {
            opcode,
            arg,
            cond,
            code,
        } => SerInstData::IntCondTrap {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            cond: cond.to_string(),
            code: code.to_string(),
        },
        InstructionData::FloatCondTrap {
            opcode,
            arg,
            cond,
            code,
        } => SerInstData::FloatCondTrap {
            opcode: opcode.to_string(),
            arg: arg.to_string(),
            cond: cond.to_string(),
            code: code.to_string(),
        },
    }
}

/// Serializable version of Cranelift IR instructions.
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct SerInst {
    pub inst_name: String,
    pub inst_data: SerInstData,
}

impl SerInst {
    pub fn new(inst: Inst, func: &Function) -> Self {
        Self {
            inst_name: inst.to_string(),
            inst_data: get_inst_data(inst, func),
        }
    }
}

/// Serializable version of Cranelift IR Blocks.
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct SerBlock {
    pub block: String,
    pub params: Vec<String>,
    pub insts: Vec<SerInst>,
}

impl SerBlock {
    pub fn new(name: String) -> Self {
        Self {
            block: name,
            params: Vec::new(),
            insts: Vec::new(),
        }
    }
}

pub fn populate_inst(func: &Function, block: Block) -> Vec<SerInst> {
    let mut ser_vec: Vec<SerInst> = Vec::new();
    let ret_iter = func.layout.block_insts(block);
    for inst in ret_iter {
        let ser_inst: SerInst = SerInst::new(inst, &func);
        ser_vec.push(ser_inst);
    }
    ser_vec
}

/// Translating Block parameters into serializable parameters.
pub fn populate_params(func: &Function, block: Block) -> Vec<String> {
    let mut ser_vec: Vec<String> = Vec::new();
    let parameters = func.dfg.block_params(block);
    for param in parameters {
        ser_vec.push(param.to_string());
    }
    ser_vec
}

/// Serializable Data Flow Graph.
#[derive(Deserialize, Serialize, Debug)]
pub struct SerDataFlowGraph {
    blocks: Vec<SerBlock>,
}

/// Serialize all parts of the Cranelift Block data structure, this includes name, parameters, and
/// instructions.
pub fn populate_blocks(func: &Function) -> Vec<SerBlock> {
    let mut block_vec: Vec<SerBlock> = Vec::new();
    for block in func.layout.blocks() {
        let mut ser_block: SerBlock = SerBlock::new(block.to_string());
        ser_block.params = populate_params(&func, block);
        ser_block.insts = populate_inst(&func, block);
        block_vec.push(ser_block);
    }
    block_vec
}

/// Serializable Cranelift IR data flow graph, including all blocks.
impl SerDataFlowGraph {
    pub fn create_new(func: &Function) -> Self {
        Self {
            blocks: populate_blocks(func),
        }
    }

    pub fn new(func: &Function) -> Self {
        Self::create_new(func)
    }
}

/// Serializable signature including function parameters and returns.
#[derive(Serialize, Deserialize, Debug)]
pub struct SerSignature {
    pub func_params: Vec<String>,
    pub func_returns: Vec<String>,
}

impl SerSignature {
    /// Creating serializable signature data structure from all Cranelift IR functions.
    fn create_new(sig: &Signature) -> Self {
        let mut params_vec: Vec<String> = Vec::new();
        let mut returns_vec: Vec<String> = Vec::new();
        for param in &sig.params {
            params_vec.push(param.to_string());
        }
        for ret in &sig.returns {
            returns_vec.push(ret.to_string());
        }
        Self {
            func_params: params_vec,
            func_returns: returns_vec,
        }
    }

    pub fn new(func: &Function) -> Self {
        Self::create_new(&func.signature)
    }
}

/// Serializable Function type, including name, signature, global values, and data flow graph.
#[derive(Serialize, Deserialize, Debug)]
pub struct SerFunction {
    pub name: String,
    pub signature: SerSignature,
    pub globals: Vec<String>,
    pub dfg: SerDataFlowGraph,
}

impl SerFunction {
    /// Creates serializable global values, as well as the functions signature, name, and data flow
    /// graph.
    fn create_new(func: &Function) -> Self {
        let mut global_vec: Vec<String> = Vec::new();
        for (glob_name, _) in func.global_values.iter() {
            global_vec.push(glob_name.to_string());
        }
        Self {
            name: func.name.to_string(),
            signature: SerSignature::new(&func),
            globals: global_vec,
            dfg: SerDataFlowGraph::new(&func),
        }
    }

    pub fn new(func: &Function) -> Self {
        Self::create_new(func)
    }
}

/// Must have SerObj for deserialization, contains all of the functions from inside the file to be
/// serialized. Files have one SerObj each, with all SerFunctions contained inside that SerObj.
#[derive(Serialize, Deserialize, Debug)]
pub struct SerObj {
    pub functions: Vec<SerFunction>,
}

impl SerObj {
    fn create_new(funcs: Vec<SerFunction>) -> Self {
        Self { functions: funcs }
    }

    pub fn new(funcs: &[Function]) -> Self {
        let mut func_vec: Vec<SerFunction> = Vec::new();
        for func in funcs {
            let ser_func: SerFunction = SerFunction::new(&func);
            func_vec.push(ser_func);
        }
        Self::create_new(func_vec)
    }
}
