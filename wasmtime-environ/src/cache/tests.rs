use super::*;
use crate::address_map::{FunctionAddressMap, InstructionAddressMap};
use crate::compilation::{Relocation, RelocationTarget};
use crate::module::{MemoryPlan, MemoryStyle, Module};
use cranelift_codegen::ir::entities::JumpTable;
use cranelift_codegen::{binemit, ir, isa, settings, ValueLocRange};
use cranelift_entity::EntityRef;
use cranelift_entity::{PrimaryMap, SecondaryMap};
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, Global, GlobalInit, Memory, SignatureIndex};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::boxed::Box;
use std::cmp::{max, min};
use std::fs;
use std::str::FromStr;
use std::vec::Vec;
use target_lexicon::triple;
use tempfile;

// Since cache system is a global thing, each test needs to be run in seperate process.
// So, init() tests are run as integration tests.
// However, caching is a private thing, an implementation detail, and needs to be tested
// from the inside of the module. Thus we have one big test here.

#[test]
fn test_write_read_cache() {
    pretty_env_logger::init();
    let dir = tempfile::tempdir().expect("Can't create temporary directory");
    conf::init(true, Some(dir.path()));
    assert!(conf::cache_enabled());
    // assumption: config init creates cache directory and returns canonicalized path
    assert_eq!(
        *conf::cache_directory(),
        fs::canonicalize(dir.path()).unwrap()
    );

    let mut rng = SmallRng::from_seed([
        0x42, 0x04, 0xF3, 0x44, 0x11, 0x22, 0x33, 0x44, 0x67, 0x68, 0xFF, 0x00, 0x44, 0x23, 0x7F,
        0x96,
    ]);

    let mut code_container = Vec::new();
    code_container.resize(0x4000, 0);
    rng.fill(&mut code_container[..]);

    let isa1 = new_isa("riscv64");
    let isa2 = new_isa("i386");
    let module1 = new_module(&mut rng);
    let module2 = new_module(&mut rng);
    let function_body_inputs1 = new_function_body_inputs(&mut rng, &code_container);
    let function_body_inputs2 = new_function_body_inputs(&mut rng, &code_container);
    let compiler1 = "test-1";
    let compiler2 = "test-2";

    let entry1 = ModuleCacheEntry::new(&module1, &function_body_inputs1, &*isa1, compiler1, false);
    assert!(entry1.mod_cache_path().is_some());
    assert!(entry1.get_data().is_none());
    let data1 = new_module_cache_data(&mut rng);
    entry1.update_data(&data1);
    assert_eq!(entry1.get_data().expect("Cache should be available"), data1);

    let entry2 = ModuleCacheEntry::new(&module2, &function_body_inputs1, &*isa1, compiler1, false);
    let data2 = new_module_cache_data(&mut rng);
    entry2.update_data(&data2);
    assert_eq!(entry1.get_data().expect("Cache should be available"), data1);
    assert_eq!(entry2.get_data().expect("Cache should be available"), data2);

    let entry3 = ModuleCacheEntry::new(&module1, &function_body_inputs2, &*isa1, compiler1, false);
    let data3 = new_module_cache_data(&mut rng);
    entry3.update_data(&data3);
    assert_eq!(entry1.get_data().expect("Cache should be available"), data1);
    assert_eq!(entry2.get_data().expect("Cache should be available"), data2);
    assert_eq!(entry3.get_data().expect("Cache should be available"), data3);

    let entry4 = ModuleCacheEntry::new(&module1, &function_body_inputs1, &*isa2, compiler1, false);
    let data4 = new_module_cache_data(&mut rng);
    entry4.update_data(&data4);
    assert_eq!(entry1.get_data().expect("Cache should be available"), data1);
    assert_eq!(entry2.get_data().expect("Cache should be available"), data2);
    assert_eq!(entry3.get_data().expect("Cache should be available"), data3);
    assert_eq!(entry4.get_data().expect("Cache should be available"), data4);

    let entry5 = ModuleCacheEntry::new(&module1, &function_body_inputs1, &*isa1, compiler2, false);
    let data5 = new_module_cache_data(&mut rng);
    entry5.update_data(&data5);
    assert_eq!(entry1.get_data().expect("Cache should be available"), data1);
    assert_eq!(entry2.get_data().expect("Cache should be available"), data2);
    assert_eq!(entry3.get_data().expect("Cache should be available"), data3);
    assert_eq!(entry4.get_data().expect("Cache should be available"), data4);
    assert_eq!(entry5.get_data().expect("Cache should be available"), data5);

    let data6 = new_module_cache_data(&mut rng);
    entry1.update_data(&data6);
    assert_eq!(entry1.get_data().expect("Cache should be available"), data6);
    assert_eq!(entry2.get_data().expect("Cache should be available"), data2);
    assert_eq!(entry3.get_data().expect("Cache should be available"), data3);
    assert_eq!(entry4.get_data().expect("Cache should be available"), data4);
    assert_eq!(entry5.get_data().expect("Cache should be available"), data5);

    assert!(data1 != data2 && data1 != data3 && data1 != data4 && data1 != data5 && data1 != data6);
}

fn new_isa(name: &str) -> Box<dyn isa::TargetIsa> {
    let shared_builder = settings::builder();
    let shared_flags = settings::Flags::new(shared_builder);
    isa::lookup(triple!(name))
        .expect("can't find specified isa")
        .finish(shared_flags)
}

fn new_module(rng: &mut impl Rng) -> Module {
    // There are way too many fields. Just fill in some of them.
    let mut m = Module::new();

    if rng.gen_bool(0.5) {
        m.signatures.push(ir::Signature {
            params: vec![],
            returns: vec![],
            call_conv: isa::CallConv::Fast,
        });
    }

    for i in 0..rng.gen_range(1, 0x8) {
        m.functions.push(SignatureIndex::new(i));
    }

    if rng.gen_bool(0.8) {
        m.memory_plans.push(MemoryPlan {
            memory: Memory {
                minimum: rng.gen(),
                maximum: rng.gen(),
                shared: rng.gen(),
            },
            style: MemoryStyle::Dynamic,
            offset_guard_size: rng.gen(),
        });
    }

    if rng.gen_bool(0.4) {
        m.globals.push(Global {
            ty: ir::Type::int(16).unwrap(),
            mutability: rng.gen(),
            initializer: GlobalInit::I32Const(rng.gen()),
        });
    }

    m
}

fn new_function_body_inputs<'data>(
    rng: &mut impl Rng,
    code_container: &'data Vec<u8>,
) -> PrimaryMap<DefinedFuncIndex, FunctionBodyData<'data>> {
    let len = code_container.len();
    let mut pos = rng.gen_range(0, code_container.len());
    (2..rng.gen_range(4, 14))
        .map(|j| {
            let (old_pos, end) = (pos, min(pos + rng.gen_range(0x10, 0x200), len));
            pos = end % len;
            FunctionBodyData {
                data: &code_container[old_pos..end],
                module_offset: (rng.next_u64() + j) as usize,
            }
        })
        .collect()
}

fn new_module_cache_data(rng: &mut impl Rng) -> ModuleCacheData {
    // WARNING: if method changed, update PartialEq impls below, too!
    let funcs = (0..rng.gen_range(0, 10))
        .map(|i| {
            let mut sm = SecondaryMap::new(); // doesn't implement from iterator
            sm.resize(i as usize * 2);
            sm.values_mut().enumerate().for_each(|(j, v)| {
                if rng.gen_bool(0.33) {
                    *v = (j as u32) * 3 / 4
                }
            });
            CodeAndJTOffsets {
                body: (0..(i * 3 / 2)).collect(),
                jt_offsets: sm,
            }
        })
        .collect();

    let relocs = (0..rng.gen_range(1, 0x10))
        .map(|i| {
            vec![
                Relocation {
                    reloc: binemit::Reloc::X86CallPCRel4,
                    reloc_target: RelocationTarget::UserFunc(FuncIndex::new(i as usize * 42)),
                    offset: i + rng.next_u32(),
                    addend: 0,
                },
                Relocation {
                    reloc: binemit::Reloc::Arm32Call,
                    reloc_target: RelocationTarget::LibCall(ir::LibCall::CeilF64),
                    offset: rng.gen_range(4, i + 55),
                    addend: (42 * i) as i64,
                },
            ]
        })
        .collect();

    let trans = (4..rng.gen_range(4, 0x10))
        .map(|i| FunctionAddressMap {
            instructions: vec![InstructionAddressMap {
                srcloc: ir::SourceLoc::new(rng.gen()),
                code_offset: rng.gen(),
                code_len: i,
            }],
            start_srcloc: ir::SourceLoc::new(rng.gen()),
            end_srcloc: ir::SourceLoc::new(rng.gen()),
            body_offset: rng.gen(),
            body_len: 0x31337,
        })
        .collect();

    let value_ranges = (4..rng.gen_range(4, 0x10))
        .map(|i| {
            (i..i + rng.gen_range(4, 8))
                .map(|k| {
                    (
                        ir::ValueLabel::new(k),
                        (0..rng.gen_range(0, 4))
                            .map(|_| ValueLocRange {
                                loc: ir::ValueLoc::Reg(rng.gen()),
                                start: rng.gen(),
                                end: rng.gen(),
                            })
                            .collect(),
                    )
                })
                .collect()
        })
        .collect();

    let stack_slots = (0..rng.gen_range(0, 0x6))
        .map(|_| {
            let mut slots = ir::StackSlots::new();
            slots.push(ir::StackSlotData {
                kind: ir::StackSlotKind::SpillSlot,
                size: rng.gen(),
                offset: rng.gen(),
            });
            slots.frame_size = rng.gen();
            slots
        })
        .collect();

    ModuleCacheData::from_tuple((
        Compilation::new(funcs),
        relocs,
        trans,
        value_ranges,
        stack_slots,
    ))
}

impl ModuleCacheEntry {
    pub fn mod_cache_path(&self) -> &Option<PathBuf> {
        &self.mod_cache_path
    }
}

// cranelift's types (including SecondaryMap) doesn't implement PartialEq
impl PartialEq for ModuleCacheData {
    fn eq(&self, other: &Self) -> bool {
        // compilation field
        if self.compilation.len() != other.compilation.len() {
            return false;
        }

        for i in (0..self.compilation.len()).map(DefinedFuncIndex::new) {
            let lhs = self.compilation.get(i);
            let rhs = other.compilation.get(i);
            if lhs.body != rhs.body || lhs.jt_offsets.get_default() != rhs.jt_offsets.get_default()
            {
                return false;
            }

            for j in (0..max(lhs.jt_offsets.len(), rhs.jt_offsets.len())).map(JumpTable::new) {
                if lhs.jt_offsets.get(j) != rhs.jt_offsets.get(j) {
                    return false;
                }
            }
        }

        // relocs
        {
            if self.relocations.len() != other.relocations.len() {
                return false;
            }
            let it_lhs = self.relocations.iter();
            let it_rhs = other.relocations.iter();
            if it_lhs.zip(it_rhs).any(|(lhs, rhs)| lhs != rhs) {
                return false;
            }
        }

        // debug symbols
        {
            if self.address_transforms.len() != other.address_transforms.len() {
                return false;
            }
            let it_lhs = self.address_transforms.iter();
            let it_rhs = other.address_transforms.iter();
            if it_lhs.zip(it_rhs).any(|(lhs, rhs)| lhs != rhs) {
                return false;
            }
        }

        true
    }
}

// binemit::Reloc doesn't implement PartialEq
impl PartialEq for Relocation {
    fn eq(&self, other: &Self) -> bool {
        self.reloc as u64 == other.reloc as u64
            && self.reloc_target == other.reloc_target
            && self.offset == other.offset
            && self.addend == other.addend
    }
}
