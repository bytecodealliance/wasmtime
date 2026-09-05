//! Canonical multi-entry regression: one optimized body, actual block offsets,
//! and no emitted analysis root. The native adapter is test-only, not the final
//! stackless gateway. Loops cover the original LICM counterexample.

use super::tests::{compile, target};
use crate::cursor::{Cursor, FuncCursor};
use crate::ir::{self, InstBuilder, MemFlagsData as MemFlags, TrapCode, types};
use alloc::vec::Vec;

const VALUES: usize = 24;
const ADDEND: i64 = 0x1234_5678_9abc_def0;
const VECTOR_ADDEND: [u64; 2] = [0xfedc_ba98_7654_3210, 0x1234_5678_9abc_def0];

fn marker(c: &mut FuncCursor<'_>, id: u32) {
    let inst = c.ins().sequence_point();
    c.func.debug_tags.set(inst, [ir::DebugTag::User(id)]);
}

fn fragment(cold: bool, looping: bool) -> ir::Function {
    let mut f = ir::Function::new();
    let a = f.dfg.make_block();
    let b = f.dfg.make_block();
    let body = f.dfg.make_block();
    for block in [a, b, body] {
        f.layout.append_block(block);
    }
    if cold {
        f.layout.set_cold(b);
    }
    let ints: Vec<_> = (0..VALUES)
        .map(|_| f.dfg.append_block_param(body, types::I64))
        .collect();
    let vectors: Vec<_> = (0..VALUES)
        .map(|_| f.dfg.append_block_param(body, types::I64X2))
        .collect();
    for (block, entry) in [(a, 0), (b, 1)] {
        let mut c = FuncCursor::new(&mut f).at_bottom(block);
        marker(&mut c, entry + 1);
        // Inputs are defined locally, never passed down from the virtual root.
        let frame = c.ins().get_pinned_reg(types::I64);
        let mut args: Vec<ir::BlockArg> = Vec::new();
        for i in 0..VALUES {
            let value = c
                .ins()
                .load(types::I64, MemFlags::trusted(), frame, (i * 8) as i32);
            args.push(c.ins().iadd_imm_s(value, ADDEND + entry as i64).into());
        }
        let constant = if looping {
            let bytes: Vec<_> = VECTOR_ADDEND.iter().flat_map(|v| v.to_le_bytes()).collect();
            let constant = c.func.dfg.constants.insert(ir::ConstantData::from(bytes));
            Some(c.ins().vconst(types::I64X2, constant))
        } else {
            None
        };
        for i in 0..VALUES {
            let value = c.ins().load(
                types::I64X2,
                MemFlags::trusted(),
                frame,
                (256 + i * 16) as i32,
            );
            let value = if let Some(constant) = constant {
                c.ins().iadd(value, constant)
            } else {
                value
            };
            args.push(value.into());
        }
        if looping {
            let remaining = c.ins().load(types::I64, MemFlags::trusted(), frame, 1808);
            let remaining = c.ins().iadd_imm_s(remaining, -1);
            c.ins().store(MemFlags::trusted(), remaining, frame, 1808);
            c.ins().brif(remaining, block, &[], body, &args);
        } else {
            c.ins().jump(body, &args);
        }
    }
    let mut c = FuncCursor::new(&mut f).at_bottom(body);
    marker(&mut c, 3);
    let frame = c.ins().get_pinned_reg(types::I64);
    for i in (0..VALUES).rev() {
        c.ins()
            .store(MemFlags::trusted(), ints[i], frame, (768 + i * 8) as i32);
        c.ins().store(
            MemFlags::trusted(),
            vectors[i],
            frame,
            (1024 + i * 16) as i32,
        );
    }
    c.ins().trap(TrapCode::unwrap_user(1));
    super::set_entries(&mut f, &[a, b]).unwrap();
    f
}

fn offsets(code: &crate::CompiledCode) -> [usize; 3] {
    let mut result = [usize::MAX; 3];
    for tag in code.buffer.debug_tags() {
        let [ir::DebugTag::User(id @ 1..=3)] = tag.tags else {
            panic!("unexpected tag")
        };
        assert_eq!(
            result[*id as usize - 1],
            usize::MAX,
            "duplicated body/entry"
        );
        result[*id as usize - 1] = tag.offset as usize;
    }
    assert!(
        result
            .iter()
            .all(|offset| *offset < code.code_buffer().len())
    );
    assert_ne!(result[0], result[1]);
    assert_ne!(result[0], result[2]);
    assert_ne!(result[1], result[2]);
    assert_eq!(code.buffer.nixe_entries.len(), 2);
    for (i, (_, offset)) in code.buffer.nixe_entries.iter().enumerate() {
        assert!(
            (*offset as usize) <= result[i],
            "entry includes allocator edits before marker"
        );
        result[i] = *offset as usize;
    }
    assert_eq!(result[0].min(result[1]), 0, "no root or dispatcher bytes");
    assert!(
        !code
            .vcode
            .as_ref()
            .unwrap()
            .lines()
            .any(|line| line == "block0:")
    );
    result
}

#[test]
fn optimized_loop_constant_is_not_initialized_in_the_virtual_root() {
    for triple in ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"] {
        let isa = target(triple, "backtracking", true);
        let mut cx = crate::Context::for_function(fragment(false, true));
        cx.optimize(&*isa, &mut cranelift_control::ControlPlane::default())
            .unwrap();
        let root = cx.func.layout.entry_block().unwrap();
        assert!(
            !cx.func
                .layout
                .block_insts(root)
                .any(|inst| cx.func.dfg.insts[inst].opcode() == ir::Opcode::Vconst)
        );
    }
}

#[test]
fn licm_still_hoists_into_an_executable_preheader() {
    for triple in ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"] {
        let isa = target(triple, "backtracking", true);
        let mut f = ir::Function::new();
        let entry = f.dfg.make_block();
        let header = f.dfg.make_block();
        let exit = f.dfg.make_block();
        for block in [entry, header, exit] {
            f.layout.append_block(block);
        }
        FuncCursor::new(&mut f)
            .at_bottom(entry)
            .ins()
            .jump(header, &[]);
        let mut c = FuncCursor::new(&mut f).at_bottom(header);
        let frame = c.ins().get_pinned_reg(types::I64);
        let constant = c.func.dfg.constants.insert(ir::ConstantData::from(
            VECTOR_ADDEND
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        ));
        let vector = c.ins().vconst(types::I64X2, constant);
        c.ins().store(MemFlags::trusted(), vector, frame, 16);
        let counter = c.ins().load(types::I64, MemFlags::trusted(), frame, 0);
        let counter = c.ins().iadd_imm_s(counter, -1);
        c.ins().store(MemFlags::trusted(), counter, frame, 0);
        c.ins().brif(counter, header, &[], exit, &[]);
        FuncCursor::new(&mut f)
            .at_bottom(exit)
            .ins()
            .trap(TrapCode::unwrap_user(1));
        super::set_entries(&mut f, &[entry]).unwrap();
        let mut cx = crate::Context::for_function(f);
        cx.optimize(&*isa, &mut cranelift_control::ControlPlane::default())
            .unwrap();
        let has_constant = |block| {
            cx.func
                .layout
                .block_insts(block)
                .any(|inst| cx.func.dfg.insts[inst].opcode() == ir::Opcode::Vconst)
        };
        assert!(has_constant(entry), "valid LICM must remain enabled");
        assert!(!has_constant(header));
        assert!(!has_constant(cx.func.layout.entry_block().unwrap()));
        compile(cx.func, &*isa).unwrap();
    }
}

#[test]
fn invalid_external_entry_boundaries_are_rejected() {
    let isa = target("x86_64-unknown-linux-gnu", "backtracking", true);
    let mut f = fragment(false, true);
    assert!(super::set_entries(&mut f, &[]).is_err());
    let ordinary = target("x86_64-unknown-linux-gnu", "backtracking", false);
    assert!(
        compile(f.clone(), &*ordinary)
            .unwrap_err()
            .contains("enable_nixe_abi")
    );
    let root = f.layout.entry_block().unwrap();
    let entry = f.nixe_entries[0];
    let root_value = f.dfg.inst_results(f.layout.first_inst(root).unwrap())[0];
    let first = f.layout.first_inst(entry).unwrap();
    // Even a valid SSA use of the analysis-only selector is not a native input.
    FuncCursor::new(&mut f).at_inst(first).ins().store(
        MemFlags::trusted(),
        root_value,
        root_value,
        0,
    );
    assert!(
        compile(f, &*isa)
            .unwrap_err()
            .contains("analysis-root value")
    );

    let mut f = fragment(false, false);
    let root = f.layout.entry_block().unwrap();
    let end = f.layout.last_inst(root).unwrap();
    FuncCursor::new(&mut f).at_inst(end).ins().sequence_point();
    assert!(
        compile(f, &*isa)
            .unwrap_err()
            .contains("unexpected computation")
    );
}

fn independent_entries(count: usize) -> ir::Function {
    let mut f = ir::Function::new();
    let mut entries = Vec::new();
    for i in 0..count {
        let block = f.dfg.make_block();
        f.layout.append_block(block);
        if i % 2 != 0 {
            f.layout.set_cold(block);
        }
        entries.push(block);
        let mut c = FuncCursor::new(&mut f).at_bottom(block);
        let frame = c.ins().get_pinned_reg(types::I64);
        let value = c.ins().iconst(types::I64, ADDEND + i as i64);
        c.ins().store(MemFlags::trusted(), value, frame, 0);
        c.ins().trap(TrapCode::unwrap_user(1));
    }
    super::set_entries(&mut f, &entries).unwrap();
    f
}

#[test]
fn one_and_many_entries_have_no_emitted_dispatcher() {
    for triple in ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"] {
        for allocator in ["single_pass", "backtracking"] {
            let isa = target(triple, allocator, true);
            for count in [1, 3, 8] {
                let f = independent_entries(count);
                let entries = f.nixe_entries.clone();
                let code = compile(f, &*isa).unwrap();
                assert_eq!(
                    code.buffer
                        .nixe_entries
                        .iter()
                        .map(|&(block, _)| block)
                        .collect::<Vec<_>>(),
                    entries
                );
                assert_eq!(
                    code.buffer
                        .nixe_entries
                        .iter()
                        .map(|&(_, offset)| offset)
                        .min(),
                    Some(0)
                );
                assert!(code.bb_edges.is_empty(), "no root edges in the emitted CFG");
                assert_eq!(code.bb_starts.len(), count);
            }
        }
    }
}

#[test]
fn two_entries_and_one_shared_body_survive_both_compiler_profiles() {
    for triple in ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"] {
        for allocator in ["single_pass", "backtracking"] {
            for cold in [false, true] {
                for looping in [false, true] {
                    let isa = target(triple, allocator, true);
                    let code = compile(fragment(cold, looping), &*isa).unwrap();
                    offsets(&code);
                    assert!(code.bb_starts.windows(2).all(|pair| pair[0] < pair[1]));
                    #[cfg(feature = "disas")]
                    code.disassemble(None, &isa.to_capstone().unwrap()).unwrap();
                    assert!(code.buffer.relocs().is_empty());
                    assert!(
                        code.buffer.frame_layout().unwrap().nixe_frame_size.unwrap()
                            > super::TRANSFER_BYTES
                    );
                }
            }
        }
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod native {
    use super::*;
    use core::ffi::c_void;

    // Test-owned ABI adapter only. The generated leaf has no prologue, calls
    // or returns; its terminal UD2 is replaced with RET in the executable test
    // copy. This does NOT prove the final stackless gateway/exit protocol.
    core::arch::global_asm!(
        ".pushsection .text",
        ".global nixe_probe_enter",
        ".hidden nixe_probe_enter",
        ".type nixe_probe_enter,@function",
        "nixe_probe_enter:",
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "sub rsp, 8",
        "mov r15, rdi",
        "mov r11, rsi",
        "mov rax, rdx",
        "mov rbx, rdx",
        "mov rcx, rdx",
        "mov rbp, rdx",
        "mov rsi, rdx",
        "mov rdi, rdx",
        "mov r8, rdx",
        "mov r9, rdx",
        "mov r10, rdx",
        "mov r12, rdx",
        "mov r13, rdx",
        "mov r14, rdx",
        "pxor xmm0, xmm0",
        "pxor xmm1, xmm1",
        "pxor xmm2, xmm2",
        "pxor xmm3, xmm3",
        "pxor xmm4, xmm4",
        "pxor xmm5, xmm5",
        "pxor xmm6, xmm6",
        "pxor xmm7, xmm7",
        "pxor xmm8, xmm8",
        "pxor xmm9, xmm9",
        "pxor xmm10, xmm10",
        "pxor xmm11, xmm11",
        "pxor xmm12, xmm12",
        "pxor xmm13, xmm13",
        "pxor xmm14, xmm14",
        "pxor xmm15, xmm15",
        "call r11",
        "add rsp, 8",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        "ret",
        ".size nixe_probe_enter, .-nixe_probe_enter",
        ".popsection",
    );

    unsafe extern "C" {
        fn nixe_probe_enter(frame: *mut u64, entry: *const u8, poison: u64);
        fn mmap(
            addr: *mut c_void,
            len: usize,
            prot: i32,
            flags: i32,
            fd: i32,
            offset: i64,
        ) -> *mut c_void;
        fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
        fn munmap(addr: *mut c_void, len: usize) -> i32;
    }

    struct Executable {
        ptr: *mut c_void,
        len: usize,
    }

    impl Executable {
        fn new(bytes: &[u8]) -> Self {
            // Linux x86-64 constants: PROT_READ|WRITE, MAP_PRIVATE|ANONYMOUS.
            // SAFETY: anonymous allocation, checked before copying; permissions
            // become RX before execution and never RWX. No external relocations.
            unsafe {
                let ptr = mmap(core::ptr::null_mut(), bytes.len(), 3, 0x22, -1, 0);
                assert_ne!(ptr as isize, -1, "mmap failed");
                let mapping = Self {
                    ptr,
                    len: bytes.len(),
                };
                core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.cast(), bytes.len());
                assert_eq!(mprotect(ptr, bytes.len(), 5), 0, "mprotect RX failed");
                mapping
            }
        }
    }

    impl Drop for Executable {
        fn drop(&mut self) {
            // SAFETY: this mapping is owned and no execution outlives it.
            unsafe { assert_eq!(munmap(self.ptr, self.len), 0) };
        }
    }

    #[repr(C, align(64))]
    struct Frame([u64; super::super::FRAME_BYTES as usize / 8]);

    #[test]
    fn native_entries_execute_without_the_virtual_root() {
        execute(false, &["single_pass", "backtracking"]);
    }

    #[test]
    fn native_loop_entries_execute_without_the_virtual_root() {
        execute(true, &["single_pass", "backtracking"]);
    }

    #[test]
    fn native_one_and_many_entries_execute_by_block_identity() {
        for allocator in ["single_pass", "backtracking"] {
            let isa = target("x86_64-unknown-linux-gnu", allocator, true);
            for count in [1, 3, 8] {
                let code = compile(independent_entries(count), &*isa).unwrap();
                let mut bytes = code.code_buffer().to_vec();
                assert!(code.buffer.relocs().is_empty());
                for trap in code.buffer.traps() {
                    let offset = trap.offset as usize;
                    assert_eq!(&bytes[offset..offset + 2], &[0x0f, 0x0b]);
                    bytes[offset..offset + 2].copy_from_slice(&[0xc3, 0x90]);
                }
                let mapping = Executable::new(&bytes);
                for (i, &(_, offset)) in code.buffer.nixe_entries.iter().enumerate() {
                    let mut frame = Frame([0; super::super::FRAME_BYTES as usize / 8]);
                    // SAFETY: same leaf fixture and SysV adapter as below.
                    unsafe {
                        nixe_probe_enter(
                            frame.0.as_mut_ptr(),
                            mapping.ptr.cast::<u8>().add(offset as usize),
                            u64::MAX,
                        );
                    }
                    assert_eq!(frame.0[0], ADDEND as u64 + i as u64);
                }
            }
        }
    }

    fn execute(looping: bool, allocators: &[&str]) {
        for allocator in allocators {
            for cold in [false, true] {
                let isa = target("x86_64-unknown-linux-gnu", allocator, true);
                let code = compile(fragment(cold, looping), &*isa).unwrap();
                let [a, b, _] = offsets(&code);
                let mut bytes = code.code_buffer().to_vec();
                assert!(code.buffer.relocs().is_empty());
                assert_eq!(code.buffer.traps().len(), 1);
                let exit = code.buffer.traps()[0].offset as usize;
                assert_eq!(&bytes[exit..exit + 2], &[0x0f, 0x0b]);
                bytes[exit..exit + 2].copy_from_slice(&[0xc3, 0x90]);
                let mapping = Executable::new(&bytes);
                for (entry, offset) in [a, b].into_iter().enumerate() {
                    for seed in [0_u64, 0x7654_3210_fedc_ba98, u64::MAX] {
                        let mut frame = Frame([seed; super::super::FRAME_BYTES as usize / 8]);
                        for i in 0..VALUES {
                            frame.0[i] = seed.wrapping_add(i as u64 * 37);
                            frame.0[32 + i * 2] = seed ^ (i as u64 * 59);
                            frame.0[33 + i * 2] = !seed ^ (i as u64 * 83);
                        }
                        frame.0[226] = 3;
                        // SAFETY: this fixture uses bounded frame offsets and
                        // no helpers; the adapter preserves SysV callee-saves.
                        unsafe {
                            nixe_probe_enter(
                                frame.0.as_mut_ptr(),
                                mapping.ptr.cast::<u8>().add(offset),
                                !seed,
                            );
                        }
                        for i in 0..VALUES {
                            assert_eq!(
                                frame.0[96 + i],
                                frame.0[i].wrapping_add(ADDEND as u64 + entry as u64),
                                "{allocator}, cold={cold}, entry={entry}, value={i}"
                            );
                            for lane in 0..2 {
                                let expected = frame.0[32 + i * 2 + lane]
                                    .wrapping_add(if looping { VECTOR_ADDEND[lane] } else { 0 });
                                assert_eq!(
                                    frame.0[128 + i * 2 + lane],
                                    expected,
                                    "{allocator}, cold={cold}, entry={entry}, vector={i}, lane={lane}"
                                );
                            }
                        }
                        assert_eq!(frame.0[226], if looping { 0 } else { 3 });
                    }
                }
            }
        }
    }
}
