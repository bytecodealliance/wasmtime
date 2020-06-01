//! Tests for the emitter
//!
//! See comments at the top of `fn x64_emit` for advice on how to create reliable test cases.
//!
//! to see stdout: cargo test -- --nocapture
//!
//! for this specific case:
//!
//! (cd cranelift/codegen && \
//! RUST_BACKTRACE=1 cargo test isa::x64::inst::test_x64_insn_encoding_and_printing -- --nocapture)

use alloc::vec::Vec;

use super::*;
use crate::isa::test_utils;

#[test]
fn test_x64_emit() {
    let rax = regs::rax();
    let rbx = regs::rbx();
    let rcx = regs::rcx();
    let rdx = regs::rdx();
    let rsi = regs::rsi();
    let rdi = regs::rdi();
    let rsp = regs::rsp();
    let rbp = regs::rbp();
    let r8 = regs::r8();
    let r9 = regs::r9();
    let r10 = regs::r10();
    let r11 = regs::r11();
    let r12 = regs::r12();
    let r13 = regs::r13();
    let r14 = regs::r14();
    let r15 = regs::r15();

    let xmm0 = regs::xmm0();
    let xmm1 = regs::xmm1();
    let xmm2 = regs::xmm2();
    let xmm3 = regs::xmm3();
    let xmm4 = regs::xmm4();
    let xmm5 = regs::xmm5();
    let xmm6 = regs::xmm6();
    let xmm7 = regs::xmm7();
    let xmm8 = regs::xmm8();
    let xmm9 = regs::xmm9();
    let xmm10 = regs::xmm10();
    let xmm11 = regs::xmm11();
    let xmm12 = regs::xmm12();
    let xmm13 = regs::xmm13();
    let xmm14 = regs::xmm14();
    let xmm15 = regs::xmm15();

    // And Writable<> versions of the same:
    let w_rax = Writable::<Reg>::from_reg(rax);
    let w_rbx = Writable::<Reg>::from_reg(rbx);
    let w_rcx = Writable::<Reg>::from_reg(rcx);
    let w_rdx = Writable::<Reg>::from_reg(rdx);
    let w_rsi = Writable::<Reg>::from_reg(rsi);
    let w_rdi = Writable::<Reg>::from_reg(rdi);
    let _w_rsp = Writable::<Reg>::from_reg(rsp);
    let _w_rbp = Writable::<Reg>::from_reg(rbp);
    let w_r8 = Writable::<Reg>::from_reg(r8);
    let w_r9 = Writable::<Reg>::from_reg(r9);
    let _w_r10 = Writable::<Reg>::from_reg(r10);
    let w_r11 = Writable::<Reg>::from_reg(r11);
    let w_r12 = Writable::<Reg>::from_reg(r12);
    let w_r13 = Writable::<Reg>::from_reg(r13);
    let w_r14 = Writable::<Reg>::from_reg(r14);
    let w_r15 = Writable::<Reg>::from_reg(r15);

    let w_xmm0 = Writable::<Reg>::from_reg(xmm0);
    let w_xmm1 = Writable::<Reg>::from_reg(xmm1);
    let w_xmm2 = Writable::<Reg>::from_reg(xmm2);
    let w_xmm3 = Writable::<Reg>::from_reg(xmm3);
    let w_xmm4 = Writable::<Reg>::from_reg(xmm4);
    let _w_xmm5 = Writable::<Reg>::from_reg(xmm5);
    let _w_xmm6 = Writable::<Reg>::from_reg(xmm6);
    let w_xmm7 = Writable::<Reg>::from_reg(xmm7);
    let w_xmm8 = Writable::<Reg>::from_reg(xmm8);
    let _w_xmm9 = Writable::<Reg>::from_reg(xmm9);
    let w_xmm10 = Writable::<Reg>::from_reg(xmm10);
    let _w_xmm11 = Writable::<Reg>::from_reg(xmm11);
    let _w_xmm12 = Writable::<Reg>::from_reg(xmm12);
    let w_xmm13 = Writable::<Reg>::from_reg(xmm13);
    let _w_xmm14 = Writable::<Reg>::from_reg(xmm14);
    let _w_xmm15 = Writable::<Reg>::from_reg(xmm15);

    let mut insns = Vec::<(Inst, &str, &str)>::new();

    // ========================================================
    // Cases aimed at checking Addr-esses: IR (Imm + Reg)
    //
    // These are just a bunch of loads with all supported (by the emitter)
    // permutations of address formats.
    //
    // Addr_IR, offset zero
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, rax), w_rdi),
        "488B38",
        "movq    0(%rax), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, rbx), w_rdi),
        "488B3B",
        "movq    0(%rbx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, rcx), w_rdi),
        "488B39",
        "movq    0(%rcx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, rdx), w_rdi),
        "488B3A",
        "movq    0(%rdx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, rbp), w_rdi),
        "488B7D00",
        "movq    0(%rbp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, rsp), w_rdi),
        "488B3C24",
        "movq    0(%rsp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, rsi), w_rdi),
        "488B3E",
        "movq    0(%rsi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, rdi), w_rdi),
        "488B3F",
        "movq    0(%rdi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, r8), w_rdi),
        "498B38",
        "movq    0(%r8), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, r9), w_rdi),
        "498B39",
        "movq    0(%r9), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, r10), w_rdi),
        "498B3A",
        "movq    0(%r10), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, r11), w_rdi),
        "498B3B",
        "movq    0(%r11), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, r12), w_rdi),
        "498B3C24",
        "movq    0(%r12), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, r13), w_rdi),
        "498B7D00",
        "movq    0(%r13), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, r14), w_rdi),
        "498B3E",
        "movq    0(%r14), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0, r15), w_rdi),
        "498B3F",
        "movq    0(%r15), %rdi",
    ));

    // ========================================================
    // Addr_IR, offset max simm8
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, rax), w_rdi),
        "488B787F",
        "movq    127(%rax), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, rbx), w_rdi),
        "488B7B7F",
        "movq    127(%rbx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, rcx), w_rdi),
        "488B797F",
        "movq    127(%rcx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, rdx), w_rdi),
        "488B7A7F",
        "movq    127(%rdx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, rbp), w_rdi),
        "488B7D7F",
        "movq    127(%rbp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, rsp), w_rdi),
        "488B7C247F",
        "movq    127(%rsp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, rsi), w_rdi),
        "488B7E7F",
        "movq    127(%rsi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, rdi), w_rdi),
        "488B7F7F",
        "movq    127(%rdi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, r8), w_rdi),
        "498B787F",
        "movq    127(%r8), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, r9), w_rdi),
        "498B797F",
        "movq    127(%r9), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, r10), w_rdi),
        "498B7A7F",
        "movq    127(%r10), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, r11), w_rdi),
        "498B7B7F",
        "movq    127(%r11), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, r12), w_rdi),
        "498B7C247F",
        "movq    127(%r12), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, r13), w_rdi),
        "498B7D7F",
        "movq    127(%r13), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, r14), w_rdi),
        "498B7E7F",
        "movq    127(%r14), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(127, r15), w_rdi),
        "498B7F7F",
        "movq    127(%r15), %rdi",
    ));

    // ========================================================
    // Addr_IR, offset min simm8
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, rax), w_rdi),
        "488B7880",
        "movq    -128(%rax), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, rbx), w_rdi),
        "488B7B80",
        "movq    -128(%rbx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, rcx), w_rdi),
        "488B7980",
        "movq    -128(%rcx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, rdx), w_rdi),
        "488B7A80",
        "movq    -128(%rdx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, rbp), w_rdi),
        "488B7D80",
        "movq    -128(%rbp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, rsp), w_rdi),
        "488B7C2480",
        "movq    -128(%rsp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, rsi), w_rdi),
        "488B7E80",
        "movq    -128(%rsi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, rdi), w_rdi),
        "488B7F80",
        "movq    -128(%rdi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, r8), w_rdi),
        "498B7880",
        "movq    -128(%r8), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, r9), w_rdi),
        "498B7980",
        "movq    -128(%r9), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, r10), w_rdi),
        "498B7A80",
        "movq    -128(%r10), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, r11), w_rdi),
        "498B7B80",
        "movq    -128(%r11), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, r12), w_rdi),
        "498B7C2480",
        "movq    -128(%r12), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, r13), w_rdi),
        "498B7D80",
        "movq    -128(%r13), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, r14), w_rdi),
        "498B7E80",
        "movq    -128(%r14), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-128i32 as u32, r15), w_rdi),
        "498B7F80",
        "movq    -128(%r15), %rdi",
    ));

    // ========================================================
    // Addr_IR, offset smallest positive simm32
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, rax), w_rdi),
        "488BB880000000",
        "movq    128(%rax), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, rbx), w_rdi),
        "488BBB80000000",
        "movq    128(%rbx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, rcx), w_rdi),
        "488BB980000000",
        "movq    128(%rcx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, rdx), w_rdi),
        "488BBA80000000",
        "movq    128(%rdx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, rbp), w_rdi),
        "488BBD80000000",
        "movq    128(%rbp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, rsp), w_rdi),
        "488BBC2480000000",
        "movq    128(%rsp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, rsi), w_rdi),
        "488BBE80000000",
        "movq    128(%rsi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, rdi), w_rdi),
        "488BBF80000000",
        "movq    128(%rdi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, r8), w_rdi),
        "498BB880000000",
        "movq    128(%r8), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, r9), w_rdi),
        "498BB980000000",
        "movq    128(%r9), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, r10), w_rdi),
        "498BBA80000000",
        "movq    128(%r10), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, r11), w_rdi),
        "498BBB80000000",
        "movq    128(%r11), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, r12), w_rdi),
        "498BBC2480000000",
        "movq    128(%r12), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, r13), w_rdi),
        "498BBD80000000",
        "movq    128(%r13), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, r14), w_rdi),
        "498BBE80000000",
        "movq    128(%r14), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(128, r15), w_rdi),
        "498BBF80000000",
        "movq    128(%r15), %rdi",
    ));

    // ========================================================
    // Addr_IR, offset smallest negative simm32
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, rax), w_rdi),
        "488BB87FFFFFFF",
        "movq    -129(%rax), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, rbx), w_rdi),
        "488BBB7FFFFFFF",
        "movq    -129(%rbx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, rcx), w_rdi),
        "488BB97FFFFFFF",
        "movq    -129(%rcx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, rdx), w_rdi),
        "488BBA7FFFFFFF",
        "movq    -129(%rdx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, rbp), w_rdi),
        "488BBD7FFFFFFF",
        "movq    -129(%rbp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, rsp), w_rdi),
        "488BBC247FFFFFFF",
        "movq    -129(%rsp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, rsi), w_rdi),
        "488BBE7FFFFFFF",
        "movq    -129(%rsi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, rdi), w_rdi),
        "488BBF7FFFFFFF",
        "movq    -129(%rdi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, r8), w_rdi),
        "498BB87FFFFFFF",
        "movq    -129(%r8), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, r9), w_rdi),
        "498BB97FFFFFFF",
        "movq    -129(%r9), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, r10), w_rdi),
        "498BBA7FFFFFFF",
        "movq    -129(%r10), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, r11), w_rdi),
        "498BBB7FFFFFFF",
        "movq    -129(%r11), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, r12), w_rdi),
        "498BBC247FFFFFFF",
        "movq    -129(%r12), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, r13), w_rdi),
        "498BBD7FFFFFFF",
        "movq    -129(%r13), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, r14), w_rdi),
        "498BBE7FFFFFFF",
        "movq    -129(%r14), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-129i32 as u32, r15), w_rdi),
        "498BBF7FFFFFFF",
        "movq    -129(%r15), %rdi",
    ));

    // ========================================================
    // Addr_IR, offset large positive simm32
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, rax), w_rdi),
        "488BB877207317",
        "movq    393420919(%rax), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, rbx), w_rdi),
        "488BBB77207317",
        "movq    393420919(%rbx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, rcx), w_rdi),
        "488BB977207317",
        "movq    393420919(%rcx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, rdx), w_rdi),
        "488BBA77207317",
        "movq    393420919(%rdx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, rbp), w_rdi),
        "488BBD77207317",
        "movq    393420919(%rbp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, rsp), w_rdi),
        "488BBC2477207317",
        "movq    393420919(%rsp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, rsi), w_rdi),
        "488BBE77207317",
        "movq    393420919(%rsi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, rdi), w_rdi),
        "488BBF77207317",
        "movq    393420919(%rdi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, r8), w_rdi),
        "498BB877207317",
        "movq    393420919(%r8), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, r9), w_rdi),
        "498BB977207317",
        "movq    393420919(%r9), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, r10), w_rdi),
        "498BBA77207317",
        "movq    393420919(%r10), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, r11), w_rdi),
        "498BBB77207317",
        "movq    393420919(%r11), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, r12), w_rdi),
        "498BBC2477207317",
        "movq    393420919(%r12), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, r13), w_rdi),
        "498BBD77207317",
        "movq    393420919(%r13), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, r14), w_rdi),
        "498BBE77207317",
        "movq    393420919(%r14), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(0x17732077, r15), w_rdi),
        "498BBF77207317",
        "movq    393420919(%r15), %rdi",
    ));

    // ========================================================
    // Addr_IR, offset large negative simm32
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, rax), w_rdi),
        "488BB8D9A6BECE",
        "movq    -826366247(%rax), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, rbx), w_rdi),
        "488BBBD9A6BECE",
        "movq    -826366247(%rbx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, rcx), w_rdi),
        "488BB9D9A6BECE",
        "movq    -826366247(%rcx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, rdx), w_rdi),
        "488BBAD9A6BECE",
        "movq    -826366247(%rdx), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, rbp), w_rdi),
        "488BBDD9A6BECE",
        "movq    -826366247(%rbp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, rsp), w_rdi),
        "488BBC24D9A6BECE",
        "movq    -826366247(%rsp), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, rsi), w_rdi),
        "488BBED9A6BECE",
        "movq    -826366247(%rsi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, rdi), w_rdi),
        "488BBFD9A6BECE",
        "movq    -826366247(%rdi), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, r8), w_rdi),
        "498BB8D9A6BECE",
        "movq    -826366247(%r8), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, r9), w_rdi),
        "498BB9D9A6BECE",
        "movq    -826366247(%r9), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, r10), w_rdi),
        "498BBAD9A6BECE",
        "movq    -826366247(%r10), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, r11), w_rdi),
        "498BBBD9A6BECE",
        "movq    -826366247(%r11), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, r12), w_rdi),
        "498BBC24D9A6BECE",
        "movq    -826366247(%r12), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, r13), w_rdi),
        "498BBDD9A6BECE",
        "movq    -826366247(%r13), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, r14), w_rdi),
        "498BBED9A6BECE",
        "movq    -826366247(%r14), %rdi",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg(-0x31415927i32 as u32, r15), w_rdi),
        "498BBFD9A6BECE",
        "movq    -826366247(%r15), %rdi",
    ));

    // ========================================================
    // Cases aimed at checking Addr-esses: IRRS (Imm + Reg + (Reg << Shift))
    // Note these don't check the case where the index reg is RSP, since we
    // don't encode any of those.
    //
    // Addr_IRRS, offset max simm8
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(127, rax, rax, 0), w_r11),
        "4C8B5C007F",
        "movq    127(%rax,%rax,1), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(127, rdi, rax, 1), w_r11),
        "4C8B5C477F",
        "movq    127(%rdi,%rax,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(127, r8, rax, 2), w_r11),
        "4D8B5C807F",
        "movq    127(%r8,%rax,4), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(127, r15, rax, 3), w_r11),
        "4D8B5CC77F",
        "movq    127(%r15,%rax,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(127, rax, rdi, 3), w_r11),
        "4C8B5CF87F",
        "movq    127(%rax,%rdi,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(127, rdi, rdi, 2), w_r11),
        "4C8B5CBF7F",
        "movq    127(%rdi,%rdi,4), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(127, r8, rdi, 1), w_r11),
        "4D8B5C787F",
        "movq    127(%r8,%rdi,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(127, r15, rdi, 0), w_r11),
        "4D8B5C3F7F",
        "movq    127(%r15,%rdi,1), %r11",
    ));

    // ========================================================
    // Addr_IRRS, offset min simm8
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(-128i32 as u32, rax, r8, 2), w_r11),
        "4E8B5C8080",
        "movq    -128(%rax,%r8,4), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(-128i32 as u32, rdi, r8, 3), w_r11),
        "4E8B5CC780",
        "movq    -128(%rdi,%r8,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(-128i32 as u32, r8, r8, 0), w_r11),
        "4F8B5C0080",
        "movq    -128(%r8,%r8,1), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(-128i32 as u32, r15, r8, 1), w_r11),
        "4F8B5C4780",
        "movq    -128(%r15,%r8,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(-128i32 as u32, rax, r15, 1), w_r11),
        "4E8B5C7880",
        "movq    -128(%rax,%r15,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(-128i32 as u32, rdi, r15, 0), w_r11),
        "4E8B5C3F80",
        "movq    -128(%rdi,%r15,1), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(-128i32 as u32, r8, r15, 3), w_r11),
        "4F8B5CF880",
        "movq    -128(%r8,%r15,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(-128i32 as u32, r15, r15, 2), w_r11),
        "4F8B5CBF80",
        "movq    -128(%r15,%r15,4), %r11",
    ));

    // ========================================================
    // Addr_IRRS, offset large positive simm32
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(0x4f6625be, rax, rax, 0), w_r11),
        "4C8B9C00BE25664F",
        "movq    1332094398(%rax,%rax,1), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(0x4f6625be, rdi, rax, 1), w_r11),
        "4C8B9C47BE25664F",
        "movq    1332094398(%rdi,%rax,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(0x4f6625be, r8, rax, 2), w_r11),
        "4D8B9C80BE25664F",
        "movq    1332094398(%r8,%rax,4), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(0x4f6625be, r15, rax, 3), w_r11),
        "4D8B9CC7BE25664F",
        "movq    1332094398(%r15,%rax,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(0x4f6625be, rax, rdi, 3), w_r11),
        "4C8B9CF8BE25664F",
        "movq    1332094398(%rax,%rdi,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(0x4f6625be, rdi, rdi, 2), w_r11),
        "4C8B9CBFBE25664F",
        "movq    1332094398(%rdi,%rdi,4), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(0x4f6625be, r8, rdi, 1), w_r11),
        "4D8B9C78BE25664F",
        "movq    1332094398(%r8,%rdi,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(0x4f6625be, r15, rdi, 0), w_r11),
        "4D8B9C3FBE25664F",
        "movq    1332094398(%r15,%rdi,1), %r11",
    ));

    // ========================================================
    // Addr_IRRS, offset large negative simm32
    insns.push((
        Inst::mov64_m_r(
            Addr::imm_reg_reg_shift(-0x264d1690i32 as u32, rax, r8, 2),
            w_r11,
        ),
        "4E8B9C8070E9B2D9",
        "movq    -642586256(%rax,%r8,4), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Addr::imm_reg_reg_shift(-0x264d1690i32 as u32, rdi, r8, 3),
            w_r11,
        ),
        "4E8B9CC770E9B2D9",
        "movq    -642586256(%rdi,%r8,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Addr::imm_reg_reg_shift(-0x264d1690i32 as u32, r8, r8, 0),
            w_r11,
        ),
        "4F8B9C0070E9B2D9",
        "movq    -642586256(%r8,%r8,1), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Addr::imm_reg_reg_shift(-0x264d1690i32 as u32, r15, r8, 1),
            w_r11,
        ),
        "4F8B9C4770E9B2D9",
        "movq    -642586256(%r15,%r8,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Addr::imm_reg_reg_shift(-0x264d1690i32 as u32, rax, r15, 1),
            w_r11,
        ),
        "4E8B9C7870E9B2D9",
        "movq    -642586256(%rax,%r15,2), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Addr::imm_reg_reg_shift(-0x264d1690i32 as u32, rdi, r15, 0),
            w_r11,
        ),
        "4E8B9C3F70E9B2D9",
        "movq    -642586256(%rdi,%r15,1), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Addr::imm_reg_reg_shift(-0x264d1690i32 as u32, r8, r15, 3),
            w_r11,
        ),
        "4F8B9CF870E9B2D9",
        "movq    -642586256(%r8,%r15,8), %r11",
    ));
    insns.push((
        Inst::mov64_m_r(
            Addr::imm_reg_reg_shift(-0x264d1690i32 as u32, r15, r15, 2),
            w_r11,
        ),
        "4F8B9CBF70E9B2D9",
        "movq    -642586256(%r15,%r15,4), %r11",
    ));

    // End of test cases for Addr
    // ========================================================

    // ========================================================
    // General tests for each insn.  Don't forget to follow the
    // guidelines commented just prior to `fn x64_emit`.
    //
    // Alu_RMI_R
    insns.push((
        Inst::alu_rmi_r(true, AluRmiROpcode::Add, RegMemImm::reg(r15), w_rdx),
        "4C01FA",
        "addq    %r15, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(false, AluRmiROpcode::Add, RegMemImm::reg(rcx), w_r8),
        "4101C8",
        "addl    %ecx, %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(false, AluRmiROpcode::Add, RegMemImm::reg(rcx), w_rsi),
        "01CE",
        "addl    %ecx, %esi",
    ));
    insns.push((
        Inst::alu_rmi_r(
            true,
            AluRmiROpcode::Add,
            RegMemImm::mem(Addr::imm_reg(99, rdi)),
            w_rdx,
        ),
        "48035763",
        "addq    99(%rdi), %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            false,
            AluRmiROpcode::Add,
            RegMemImm::mem(Addr::imm_reg(99, rdi)),
            w_r8,
        ),
        "44034763",
        "addl    99(%rdi), %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(
            false,
            AluRmiROpcode::Add,
            RegMemImm::mem(Addr::imm_reg(99, rdi)),
            w_rsi,
        ),
        "037763",
        "addl    99(%rdi), %esi",
    ));
    insns.push((
        Inst::alu_rmi_r(
            true,
            AluRmiROpcode::Add,
            RegMemImm::imm(-127i32 as u32),
            w_rdx,
        ),
        "4883C281",
        "addq    $-127, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            true,
            AluRmiROpcode::Add,
            RegMemImm::imm(-129i32 as u32),
            w_rdx,
        ),
        "4881C27FFFFFFF",
        "addq    $-129, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(true, AluRmiROpcode::Add, RegMemImm::imm(76543210), w_rdx),
        "4881C2EAF48F04",
        "addq    $76543210, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            false,
            AluRmiROpcode::Add,
            RegMemImm::imm(-127i32 as u32),
            w_r8,
        ),
        "4183C081",
        "addl    $-127, %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(
            false,
            AluRmiROpcode::Add,
            RegMemImm::imm(-129i32 as u32),
            w_r8,
        ),
        "4181C07FFFFFFF",
        "addl    $-129, %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(
            false,
            AluRmiROpcode::Add,
            RegMemImm::imm(-76543210i32 as u32),
            w_r8,
        ),
        "4181C0160B70FB",
        "addl    $-76543210, %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(
            false,
            AluRmiROpcode::Add,
            RegMemImm::imm(-127i32 as u32),
            w_rsi,
        ),
        "83C681",
        "addl    $-127, %esi",
    ));
    insns.push((
        Inst::alu_rmi_r(
            false,
            AluRmiROpcode::Add,
            RegMemImm::imm(-129i32 as u32),
            w_rsi,
        ),
        "81C67FFFFFFF",
        "addl    $-129, %esi",
    ));
    insns.push((
        Inst::alu_rmi_r(false, AluRmiROpcode::Add, RegMemImm::imm(76543210), w_rsi),
        "81C6EAF48F04",
        "addl    $76543210, %esi",
    ));
    // This is pretty feeble
    insns.push((
        Inst::alu_rmi_r(true, AluRmiROpcode::Sub, RegMemImm::reg(r15), w_rdx),
        "4C29FA",
        "subq    %r15, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(true, AluRmiROpcode::And, RegMemImm::reg(r15), w_rdx),
        "4C21FA",
        "andq    %r15, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(true, AluRmiROpcode::Or, RegMemImm::reg(r15), w_rdx),
        "4C09FA",
        "orq     %r15, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(true, AluRmiROpcode::Xor, RegMemImm::reg(r15), w_rdx),
        "4C31FA",
        "xorq    %r15, %rdx",
    ));
    // Test all mul cases, though
    insns.push((
        Inst::alu_rmi_r(true, AluRmiROpcode::Mul, RegMemImm::reg(r15), w_rdx),
        "490FAFD7",
        "imulq   %r15, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(false, AluRmiROpcode::Mul, RegMemImm::reg(rcx), w_r8),
        "440FAFC1",
        "imull   %ecx, %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(false, AluRmiROpcode::Mul, RegMemImm::reg(rcx), w_rsi),
        "0FAFF1",
        "imull   %ecx, %esi",
    ));
    insns.push((
        Inst::alu_rmi_r(
            true,
            AluRmiROpcode::Mul,
            RegMemImm::mem(Addr::imm_reg(99, rdi)),
            w_rdx,
        ),
        "480FAF5763",
        "imulq   99(%rdi), %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            false,
            AluRmiROpcode::Mul,
            RegMemImm::mem(Addr::imm_reg(99, rdi)),
            w_r8,
        ),
        "440FAF4763",
        "imull   99(%rdi), %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(
            false,
            AluRmiROpcode::Mul,
            RegMemImm::mem(Addr::imm_reg(99, rdi)),
            w_rsi,
        ),
        "0FAF7763",
        "imull   99(%rdi), %esi",
    ));
    insns.push((
        Inst::alu_rmi_r(
            true,
            AluRmiROpcode::Mul,
            RegMemImm::imm(-127i32 as u32),
            w_rdx,
        ),
        "486BD281",
        "imulq   $-127, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            true,
            AluRmiROpcode::Mul,
            RegMemImm::imm(-129i32 as u32),
            w_rdx,
        ),
        "4869D27FFFFFFF",
        "imulq   $-129, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(true, AluRmiROpcode::Mul, RegMemImm::imm(76543210), w_rdx),
        "4869D2EAF48F04",
        "imulq   $76543210, %rdx",
    ));
    insns.push((
        Inst::alu_rmi_r(
            false,
            AluRmiROpcode::Mul,
            RegMemImm::imm(-127i32 as u32),
            w_r8,
        ),
        "456BC081",
        "imull   $-127, %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(
            false,
            AluRmiROpcode::Mul,
            RegMemImm::imm(-129i32 as u32),
            w_r8,
        ),
        "4569C07FFFFFFF",
        "imull   $-129, %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(
            false,
            AluRmiROpcode::Mul,
            RegMemImm::imm(-76543210i32 as u32),
            w_r8,
        ),
        "4569C0160B70FB",
        "imull   $-76543210, %r8d",
    ));
    insns.push((
        Inst::alu_rmi_r(
            false,
            AluRmiROpcode::Mul,
            RegMemImm::imm(-127i32 as u32),
            w_rsi,
        ),
        "6BF681",
        "imull   $-127, %esi",
    ));
    insns.push((
        Inst::alu_rmi_r(
            false,
            AluRmiROpcode::Mul,
            RegMemImm::imm(-129i32 as u32),
            w_rsi,
        ),
        "69F67FFFFFFF",
        "imull   $-129, %esi",
    ));
    insns.push((
        Inst::alu_rmi_r(false, AluRmiROpcode::Mul, RegMemImm::imm(76543210), w_rsi),
        "69F6EAF48F04",
        "imull   $76543210, %esi",
    ));

    // ========================================================
    // Imm_R
    //
    insns.push((
        Inst::imm_r(false, 1234567, w_r14),
        "41BE87D61200",
        "movl    $1234567, %r14d",
    ));
    insns.push((
        Inst::imm_r(false, -126i64 as u64, w_r14),
        "41BE82FFFFFF",
        "movl    $-126, %r14d",
    ));
    insns.push((
        Inst::imm_r(true, 1234567898765, w_r14),
        "49BE8D26FB711F010000",
        "movabsq $1234567898765, %r14",
    ));
    insns.push((
        Inst::imm_r(true, -126i64 as u64, w_r14),
        "49BE82FFFFFFFFFFFFFF",
        "movabsq $-126, %r14",
    ));
    insns.push((
        Inst::imm_r(false, 1234567, w_rcx),
        "B987D61200",
        "movl    $1234567, %ecx",
    ));
    insns.push((
        Inst::imm_r(false, -126i64 as u64, w_rcx),
        "B982FFFFFF",
        "movl    $-126, %ecx",
    ));
    insns.push((
        Inst::imm_r(true, 1234567898765, w_rsi),
        "48BE8D26FB711F010000",
        "movabsq $1234567898765, %rsi",
    ));
    insns.push((
        Inst::imm_r(true, -126i64 as u64, w_rbx),
        "48BB82FFFFFFFFFFFFFF",
        "movabsq $-126, %rbx",
    ));

    // ========================================================
    // Mov_R_R
    insns.push((
        Inst::mov_r_r(false, rbx, w_rsi),
        "89DE",
        "movl    %ebx, %esi",
    ));
    insns.push((
        Inst::mov_r_r(false, rbx, w_r9),
        "4189D9",
        "movl    %ebx, %r9d",
    ));
    insns.push((
        Inst::mov_r_r(false, r11, w_rsi),
        "4489DE",
        "movl    %r11d, %esi",
    ));
    insns.push((
        Inst::mov_r_r(false, r12, w_r9),
        "4589E1",
        "movl    %r12d, %r9d",
    ));
    insns.push((
        Inst::mov_r_r(true, rbx, w_rsi),
        "4889DE",
        "movq    %rbx, %rsi",
    ));
    insns.push((
        Inst::mov_r_r(true, rbx, w_r9),
        "4989D9",
        "movq    %rbx, %r9",
    ));
    insns.push((
        Inst::mov_r_r(true, r11, w_rsi),
        "4C89DE",
        "movq    %r11, %rsi",
    ));
    insns.push((
        Inst::mov_r_r(true, r12, w_r9),
        "4D89E1",
        "movq    %r12, %r9",
    ));

    // ========================================================
    // MovZX_M_R
    insns.push((
        Inst::movzx_m_r(ExtMode::BL, Addr::imm_reg(-7i32 as u32, rcx), w_rsi),
        "0FB671F9",
        "movzbl  -7(%rcx), %esi",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::BL, Addr::imm_reg(-7i32 as u32, r8), w_rbx),
        "410FB658F9",
        "movzbl  -7(%r8), %ebx",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::BL, Addr::imm_reg(-7i32 as u32, r10), w_r9),
        "450FB64AF9",
        "movzbl  -7(%r10), %r9d",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::BL, Addr::imm_reg(-7i32 as u32, r11), w_rdx),
        "410FB653F9",
        "movzbl  -7(%r11), %edx",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::BQ, Addr::imm_reg(-7i32 as u32, rcx), w_rsi),
        "480FB671F9",
        "movzbq  -7(%rcx), %rsi",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::BQ, Addr::imm_reg(-7i32 as u32, r8), w_rbx),
        "490FB658F9",
        "movzbq  -7(%r8), %rbx",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::BQ, Addr::imm_reg(-7i32 as u32, r10), w_r9),
        "4D0FB64AF9",
        "movzbq  -7(%r10), %r9",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::BQ, Addr::imm_reg(-7i32 as u32, r11), w_rdx),
        "490FB653F9",
        "movzbq  -7(%r11), %rdx",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::WL, Addr::imm_reg(-7i32 as u32, rcx), w_rsi),
        "0FB771F9",
        "movzwl  -7(%rcx), %esi",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::WL, Addr::imm_reg(-7i32 as u32, r8), w_rbx),
        "410FB758F9",
        "movzwl  -7(%r8), %ebx",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::WL, Addr::imm_reg(-7i32 as u32, r10), w_r9),
        "450FB74AF9",
        "movzwl  -7(%r10), %r9d",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::WL, Addr::imm_reg(-7i32 as u32, r11), w_rdx),
        "410FB753F9",
        "movzwl  -7(%r11), %edx",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::WQ, Addr::imm_reg(-7i32 as u32, rcx), w_rsi),
        "480FB771F9",
        "movzwq  -7(%rcx), %rsi",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::WQ, Addr::imm_reg(-7i32 as u32, r8), w_rbx),
        "490FB758F9",
        "movzwq  -7(%r8), %rbx",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::WQ, Addr::imm_reg(-7i32 as u32, r10), w_r9),
        "4D0FB74AF9",
        "movzwq  -7(%r10), %r9",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::WQ, Addr::imm_reg(-7i32 as u32, r11), w_rdx),
        "490FB753F9",
        "movzwq  -7(%r11), %rdx",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::LQ, Addr::imm_reg(-7i32 as u32, rcx), w_rsi),
        "8B71F9",
        "movl    -7(%rcx), %esi",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::LQ, Addr::imm_reg(-7i32 as u32, r8), w_rbx),
        "418B58F9",
        "movl    -7(%r8), %ebx",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::LQ, Addr::imm_reg(-7i32 as u32, r10), w_r9),
        "458B4AF9",
        "movl    -7(%r10), %r9d",
    ));
    insns.push((
        Inst::movzx_m_r(ExtMode::LQ, Addr::imm_reg(-7i32 as u32, r11), w_rdx),
        "418B53F9",
        "movl    -7(%r11), %edx",
    ));

    // ========================================================
    // Mov64_M_R
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(179, rax, rbx, 0), w_rcx),
        "488B8C18B3000000",
        "movq    179(%rax,%rbx,1), %rcx",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(179, rax, rbx, 0), w_r8),
        "4C8B8418B3000000",
        "movq    179(%rax,%rbx,1), %r8",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(179, rax, r9, 0), w_rcx),
        "4A8B8C08B3000000",
        "movq    179(%rax,%r9,1), %rcx",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(179, rax, r9, 0), w_r8),
        "4E8B8408B3000000",
        "movq    179(%rax,%r9,1), %r8",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(179, r10, rbx, 0), w_rcx),
        "498B8C1AB3000000",
        "movq    179(%r10,%rbx,1), %rcx",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(179, r10, rbx, 0), w_r8),
        "4D8B841AB3000000",
        "movq    179(%r10,%rbx,1), %r8",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(179, r10, r9, 0), w_rcx),
        "4B8B8C0AB3000000",
        "movq    179(%r10,%r9,1), %rcx",
    ));
    insns.push((
        Inst::mov64_m_r(Addr::imm_reg_reg_shift(179, r10, r9, 0), w_r8),
        "4F8B840AB3000000",
        "movq    179(%r10,%r9,1), %r8",
    ));

    // ========================================================
    // MovSX_M_R
    insns.push((
        Inst::movsx_m_r(ExtMode::BL, Addr::imm_reg(-7i32 as u32, rcx), w_rsi),
        "0FBE71F9",
        "movsbl  -7(%rcx), %esi",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::BL, Addr::imm_reg(-7i32 as u32, r8), w_rbx),
        "410FBE58F9",
        "movsbl  -7(%r8), %ebx",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::BL, Addr::imm_reg(-7i32 as u32, r10), w_r9),
        "450FBE4AF9",
        "movsbl  -7(%r10), %r9d",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::BL, Addr::imm_reg(-7i32 as u32, r11), w_rdx),
        "410FBE53F9",
        "movsbl  -7(%r11), %edx",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::BQ, Addr::imm_reg(-7i32 as u32, rcx), w_rsi),
        "480FBE71F9",
        "movsbq  -7(%rcx), %rsi",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::BQ, Addr::imm_reg(-7i32 as u32, r8), w_rbx),
        "490FBE58F9",
        "movsbq  -7(%r8), %rbx",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::BQ, Addr::imm_reg(-7i32 as u32, r10), w_r9),
        "4D0FBE4AF9",
        "movsbq  -7(%r10), %r9",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::BQ, Addr::imm_reg(-7i32 as u32, r11), w_rdx),
        "490FBE53F9",
        "movsbq  -7(%r11), %rdx",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::WL, Addr::imm_reg(-7i32 as u32, rcx), w_rsi),
        "0FBF71F9",
        "movswl  -7(%rcx), %esi",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::WL, Addr::imm_reg(-7i32 as u32, r8), w_rbx),
        "410FBF58F9",
        "movswl  -7(%r8), %ebx",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::WL, Addr::imm_reg(-7i32 as u32, r10), w_r9),
        "450FBF4AF9",
        "movswl  -7(%r10), %r9d",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::WL, Addr::imm_reg(-7i32 as u32, r11), w_rdx),
        "410FBF53F9",
        "movswl  -7(%r11), %edx",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::WQ, Addr::imm_reg(-7i32 as u32, rcx), w_rsi),
        "480FBF71F9",
        "movswq  -7(%rcx), %rsi",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::WQ, Addr::imm_reg(-7i32 as u32, r8), w_rbx),
        "490FBF58F9",
        "movswq  -7(%r8), %rbx",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::WQ, Addr::imm_reg(-7i32 as u32, r10), w_r9),
        "4D0FBF4AF9",
        "movswq  -7(%r10), %r9",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::WQ, Addr::imm_reg(-7i32 as u32, r11), w_rdx),
        "490FBF53F9",
        "movswq  -7(%r11), %rdx",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::LQ, Addr::imm_reg(-7i32 as u32, rcx), w_rsi),
        "486371F9",
        "movslq  -7(%rcx), %rsi",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::LQ, Addr::imm_reg(-7i32 as u32, r8), w_rbx),
        "496358F9",
        "movslq  -7(%r8), %rbx",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::LQ, Addr::imm_reg(-7i32 as u32, r10), w_r9),
        "4D634AF9",
        "movslq  -7(%r10), %r9",
    ));
    insns.push((
        Inst::movsx_m_r(ExtMode::LQ, Addr::imm_reg(-7i32 as u32, r11), w_rdx),
        "496353F9",
        "movslq  -7(%r11), %rdx",
    ));

    // ========================================================
    // Mov_R_M.  Byte stores are tricky.  Check everything carefully.
    insns.push((
        Inst::mov_r_m(8, rax, Addr::imm_reg(99, rdi)),
        "48894763",
        "movq    %rax, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(8, rbx, Addr::imm_reg(99, r8)),
        "49895863",
        "movq    %rbx, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(8, rcx, Addr::imm_reg(99, rsi)),
        "48894E63",
        "movq    %rcx, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(8, rdx, Addr::imm_reg(99, r9)),
        "49895163",
        "movq    %rdx, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(8, rsi, Addr::imm_reg(99, rax)),
        "48897063",
        "movq    %rsi, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(8, rdi, Addr::imm_reg(99, r15)),
        "49897F63",
        "movq    %rdi, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(8, rsp, Addr::imm_reg(99, rcx)),
        "48896163",
        "movq    %rsp, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(8, rbp, Addr::imm_reg(99, r14)),
        "49896E63",
        "movq    %rbp, 99(%r14)",
    ));
    insns.push((
        Inst::mov_r_m(8, r8, Addr::imm_reg(99, rdi)),
        "4C894763",
        "movq    %r8, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(8, r9, Addr::imm_reg(99, r8)),
        "4D894863",
        "movq    %r9, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(8, r10, Addr::imm_reg(99, rsi)),
        "4C895663",
        "movq    %r10, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(8, r11, Addr::imm_reg(99, r9)),
        "4D895963",
        "movq    %r11, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(8, r12, Addr::imm_reg(99, rax)),
        "4C896063",
        "movq    %r12, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(8, r13, Addr::imm_reg(99, r15)),
        "4D896F63",
        "movq    %r13, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(8, r14, Addr::imm_reg(99, rcx)),
        "4C897163",
        "movq    %r14, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(8, r15, Addr::imm_reg(99, r14)),
        "4D897E63",
        "movq    %r15, 99(%r14)",
    ));
    //
    insns.push((
        Inst::mov_r_m(4, rax, Addr::imm_reg(99, rdi)),
        "894763",
        "movl    %eax, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(4, rbx, Addr::imm_reg(99, r8)),
        "41895863",
        "movl    %ebx, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(4, rcx, Addr::imm_reg(99, rsi)),
        "894E63",
        "movl    %ecx, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(4, rdx, Addr::imm_reg(99, r9)),
        "41895163",
        "movl    %edx, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(4, rsi, Addr::imm_reg(99, rax)),
        "897063",
        "movl    %esi, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(4, rdi, Addr::imm_reg(99, r15)),
        "41897F63",
        "movl    %edi, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(4, rsp, Addr::imm_reg(99, rcx)),
        "896163",
        "movl    %esp, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(4, rbp, Addr::imm_reg(99, r14)),
        "41896E63",
        "movl    %ebp, 99(%r14)",
    ));
    insns.push((
        Inst::mov_r_m(4, r8, Addr::imm_reg(99, rdi)),
        "44894763",
        "movl    %r8d, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(4, r9, Addr::imm_reg(99, r8)),
        "45894863",
        "movl    %r9d, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(4, r10, Addr::imm_reg(99, rsi)),
        "44895663",
        "movl    %r10d, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(4, r11, Addr::imm_reg(99, r9)),
        "45895963",
        "movl    %r11d, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(4, r12, Addr::imm_reg(99, rax)),
        "44896063",
        "movl    %r12d, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(4, r13, Addr::imm_reg(99, r15)),
        "45896F63",
        "movl    %r13d, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(4, r14, Addr::imm_reg(99, rcx)),
        "44897163",
        "movl    %r14d, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(4, r15, Addr::imm_reg(99, r14)),
        "45897E63",
        "movl    %r15d, 99(%r14)",
    ));
    //
    insns.push((
        Inst::mov_r_m(2, rax, Addr::imm_reg(99, rdi)),
        "66894763",
        "movw    %ax, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(2, rbx, Addr::imm_reg(99, r8)),
        "6641895863",
        "movw    %bx, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(2, rcx, Addr::imm_reg(99, rsi)),
        "66894E63",
        "movw    %cx, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(2, rdx, Addr::imm_reg(99, r9)),
        "6641895163",
        "movw    %dx, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(2, rsi, Addr::imm_reg(99, rax)),
        "66897063",
        "movw    %si, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(2, rdi, Addr::imm_reg(99, r15)),
        "6641897F63",
        "movw    %di, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(2, rsp, Addr::imm_reg(99, rcx)),
        "66896163",
        "movw    %sp, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(2, rbp, Addr::imm_reg(99, r14)),
        "6641896E63",
        "movw    %bp, 99(%r14)",
    ));
    insns.push((
        Inst::mov_r_m(2, r8, Addr::imm_reg(99, rdi)),
        "6644894763",
        "movw    %r8w, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(2, r9, Addr::imm_reg(99, r8)),
        "6645894863",
        "movw    %r9w, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(2, r10, Addr::imm_reg(99, rsi)),
        "6644895663",
        "movw    %r10w, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(2, r11, Addr::imm_reg(99, r9)),
        "6645895963",
        "movw    %r11w, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(2, r12, Addr::imm_reg(99, rax)),
        "6644896063",
        "movw    %r12w, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(2, r13, Addr::imm_reg(99, r15)),
        "6645896F63",
        "movw    %r13w, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(2, r14, Addr::imm_reg(99, rcx)),
        "6644897163",
        "movw    %r14w, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(2, r15, Addr::imm_reg(99, r14)),
        "6645897E63",
        "movw    %r15w, 99(%r14)",
    ));
    //
    insns.push((
        Inst::mov_r_m(1, rax, Addr::imm_reg(99, rdi)),
        "884763",
        "movb    %al, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(1, rbx, Addr::imm_reg(99, r8)),
        "41885863",
        "movb    %bl, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(1, rcx, Addr::imm_reg(99, rsi)),
        "884E63",
        "movb    %cl, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(1, rdx, Addr::imm_reg(99, r9)),
        "41885163",
        "movb    %dl, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(1, rsi, Addr::imm_reg(99, rax)),
        "40887063",
        "movb    %sil, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(1, rdi, Addr::imm_reg(99, r15)),
        "41887F63",
        "movb    %dil, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(1, rsp, Addr::imm_reg(99, rcx)),
        "40886163",
        "movb    %spl, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(1, rbp, Addr::imm_reg(99, r14)),
        "41886E63",
        "movb    %bpl, 99(%r14)",
    ));
    insns.push((
        Inst::mov_r_m(1, r8, Addr::imm_reg(99, rdi)),
        "44884763",
        "movb    %r8b, 99(%rdi)",
    ));
    insns.push((
        Inst::mov_r_m(1, r9, Addr::imm_reg(99, r8)),
        "45884863",
        "movb    %r9b, 99(%r8)",
    ));
    insns.push((
        Inst::mov_r_m(1, r10, Addr::imm_reg(99, rsi)),
        "44885663",
        "movb    %r10b, 99(%rsi)",
    ));
    insns.push((
        Inst::mov_r_m(1, r11, Addr::imm_reg(99, r9)),
        "45885963",
        "movb    %r11b, 99(%r9)",
    ));
    insns.push((
        Inst::mov_r_m(1, r12, Addr::imm_reg(99, rax)),
        "44886063",
        "movb    %r12b, 99(%rax)",
    ));
    insns.push((
        Inst::mov_r_m(1, r13, Addr::imm_reg(99, r15)),
        "45886F63",
        "movb    %r13b, 99(%r15)",
    ));
    insns.push((
        Inst::mov_r_m(1, r14, Addr::imm_reg(99, rcx)),
        "44887163",
        "movb    %r14b, 99(%rcx)",
    ));
    insns.push((
        Inst::mov_r_m(1, r15, Addr::imm_reg(99, r14)),
        "45887E63",
        "movb    %r15b, 99(%r14)",
    ));

    // ========================================================
    // Shift_R
    insns.push((
        Inst::shift_r(false, ShiftKind::Left, None, w_rdi),
        "D3E7",
        "shll    %cl, %edi",
    ));
    insns.push((
        Inst::shift_r(false, ShiftKind::Left, None, w_r12),
        "41D3E4",
        "shll    %cl, %r12d",
    ));
    insns.push((
        Inst::shift_r(false, ShiftKind::Left, Some(2), w_r8),
        "41C1E002",
        "shll    $2, %r8d",
    ));
    insns.push((
        Inst::shift_r(false, ShiftKind::Left, Some(31), w_r13),
        "41C1E51F",
        "shll    $31, %r13d",
    ));
    insns.push((
        Inst::shift_r(true, ShiftKind::Left, None, w_r13),
        "49D3E5",
        "shlq    %cl, %r13",
    ));
    insns.push((
        Inst::shift_r(true, ShiftKind::Left, None, w_rdi),
        "48D3E7",
        "shlq    %cl, %rdi",
    ));
    insns.push((
        Inst::shift_r(true, ShiftKind::Left, Some(2), w_r8),
        "49C1E002",
        "shlq    $2, %r8",
    ));
    insns.push((
        Inst::shift_r(true, ShiftKind::Left, Some(3), w_rbx),
        "48C1E303",
        "shlq    $3, %rbx",
    ));
    insns.push((
        Inst::shift_r(true, ShiftKind::Left, Some(63), w_r13),
        "49C1E53F",
        "shlq    $63, %r13",
    ));
    insns.push((
        Inst::shift_r(false, ShiftKind::RightZ, None, w_rdi),
        "D3EF",
        "shrl    %cl, %edi",
    ));
    insns.push((
        Inst::shift_r(false, ShiftKind::RightZ, Some(2), w_r8),
        "41C1E802",
        "shrl    $2, %r8d",
    ));
    insns.push((
        Inst::shift_r(false, ShiftKind::RightZ, Some(31), w_r13),
        "41C1ED1F",
        "shrl    $31, %r13d",
    ));
    insns.push((
        Inst::shift_r(true, ShiftKind::RightZ, None, w_rdi),
        "48D3EF",
        "shrq    %cl, %rdi",
    ));
    insns.push((
        Inst::shift_r(true, ShiftKind::RightZ, Some(2), w_r8),
        "49C1E802",
        "shrq    $2, %r8",
    ));
    insns.push((
        Inst::shift_r(true, ShiftKind::RightZ, Some(63), w_r13),
        "49C1ED3F",
        "shrq    $63, %r13",
    ));
    insns.push((
        Inst::shift_r(false, ShiftKind::RightS, None, w_rdi),
        "D3FF",
        "sarl    %cl, %edi",
    ));
    insns.push((
        Inst::shift_r(false, ShiftKind::RightS, Some(2), w_r8),
        "41C1F802",
        "sarl    $2, %r8d",
    ));
    insns.push((
        Inst::shift_r(false, ShiftKind::RightS, Some(31), w_r13),
        "41C1FD1F",
        "sarl    $31, %r13d",
    ));
    insns.push((
        Inst::shift_r(true, ShiftKind::RightS, None, w_rdi),
        "48D3FF",
        "sarq    %cl, %rdi",
    ));
    insns.push((
        Inst::shift_r(true, ShiftKind::RightS, Some(2), w_r8),
        "49C1F802",
        "sarq    $2, %r8",
    ));
    insns.push((
        Inst::shift_r(true, ShiftKind::RightS, Some(63), w_r13),
        "49C1FD3F",
        "sarq    $63, %r13",
    ));

    // ========================================================
    // CmpRMIR
    insns.push((
        Inst::cmp_rmi_r(8, RegMemImm::reg(r15), rdx),
        "4C39FA",
        "cmpq    %r15, %rdx",
    ));
    insns.push((
        Inst::cmp_rmi_r(8, RegMemImm::reg(rcx), r8),
        "4939C8",
        "cmpq    %rcx, %r8",
    ));
    insns.push((
        Inst::cmp_rmi_r(8, RegMemImm::reg(rcx), rsi),
        "4839CE",
        "cmpq    %rcx, %rsi",
    ));
    insns.push((
        Inst::cmp_rmi_r(8, RegMemImm::mem(Addr::imm_reg(99, rdi)), rdx),
        "483B5763",
        "cmpq    99(%rdi), %rdx",
    ));
    insns.push((
        Inst::cmp_rmi_r(8, RegMemImm::mem(Addr::imm_reg(99, rdi)), r8),
        "4C3B4763",
        "cmpq    99(%rdi), %r8",
    ));
    insns.push((
        Inst::cmp_rmi_r(8, RegMemImm::mem(Addr::imm_reg(99, rdi)), rsi),
        "483B7763",
        "cmpq    99(%rdi), %rsi",
    ));
    insns.push((
        Inst::cmp_rmi_r(8, RegMemImm::imm(76543210), rdx),
        "4881FAEAF48F04",
        "cmpq    $76543210, %rdx",
    ));
    insns.push((
        Inst::cmp_rmi_r(8, RegMemImm::imm(-76543210i32 as u32), r8),
        "4981F8160B70FB",
        "cmpq    $-76543210, %r8",
    ));
    insns.push((
        Inst::cmp_rmi_r(8, RegMemImm::imm(76543210), rsi),
        "4881FEEAF48F04",
        "cmpq    $76543210, %rsi",
    ));
    //
    insns.push((
        Inst::cmp_rmi_r(4, RegMemImm::reg(r15), rdx),
        "4439FA",
        "cmpl    %r15d, %edx",
    ));
    insns.push((
        Inst::cmp_rmi_r(4, RegMemImm::reg(rcx), r8),
        "4139C8",
        "cmpl    %ecx, %r8d",
    ));
    insns.push((
        Inst::cmp_rmi_r(4, RegMemImm::reg(rcx), rsi),
        "39CE",
        "cmpl    %ecx, %esi",
    ));
    insns.push((
        Inst::cmp_rmi_r(4, RegMemImm::mem(Addr::imm_reg(99, rdi)), rdx),
        "3B5763",
        "cmpl    99(%rdi), %edx",
    ));
    insns.push((
        Inst::cmp_rmi_r(4, RegMemImm::mem(Addr::imm_reg(99, rdi)), r8),
        "443B4763",
        "cmpl    99(%rdi), %r8d",
    ));
    insns.push((
        Inst::cmp_rmi_r(4, RegMemImm::mem(Addr::imm_reg(99, rdi)), rsi),
        "3B7763",
        "cmpl    99(%rdi), %esi",
    ));
    insns.push((
        Inst::cmp_rmi_r(4, RegMemImm::imm(76543210), rdx),
        "81FAEAF48F04",
        "cmpl    $76543210, %edx",
    ));
    insns.push((
        Inst::cmp_rmi_r(4, RegMemImm::imm(-76543210i32 as u32), r8),
        "4181F8160B70FB",
        "cmpl    $-76543210, %r8d",
    ));
    insns.push((
        Inst::cmp_rmi_r(4, RegMemImm::imm(76543210), rsi),
        "81FEEAF48F04",
        "cmpl    $76543210, %esi",
    ));
    //
    insns.push((
        Inst::cmp_rmi_r(2, RegMemImm::reg(r15), rdx),
        "664439FA",
        "cmpw    %r15w, %dx",
    ));
    insns.push((
        Inst::cmp_rmi_r(2, RegMemImm::reg(rcx), r8),
        "664139C8",
        "cmpw    %cx, %r8w",
    ));
    insns.push((
        Inst::cmp_rmi_r(2, RegMemImm::reg(rcx), rsi),
        "6639CE",
        "cmpw    %cx, %si",
    ));
    insns.push((
        Inst::cmp_rmi_r(2, RegMemImm::mem(Addr::imm_reg(99, rdi)), rdx),
        "663B5763",
        "cmpw    99(%rdi), %dx",
    ));
    insns.push((
        Inst::cmp_rmi_r(2, RegMemImm::mem(Addr::imm_reg(99, rdi)), r8),
        "66443B4763",
        "cmpw    99(%rdi), %r8w",
    ));
    insns.push((
        Inst::cmp_rmi_r(2, RegMemImm::mem(Addr::imm_reg(99, rdi)), rsi),
        "663B7763",
        "cmpw    99(%rdi), %si",
    ));
    insns.push((
        Inst::cmp_rmi_r(2, RegMemImm::imm(23210), rdx),
        "6681FAAA5A",
        "cmpw    $23210, %dx",
    ));
    insns.push((
        Inst::cmp_rmi_r(2, RegMemImm::imm(-7654i32 as u32), r8),
        "664181F81AE2",
        "cmpw    $-7654, %r8w",
    ));
    insns.push((
        Inst::cmp_rmi_r(2, RegMemImm::imm(7654), rsi),
        "6681FEE61D",
        "cmpw    $7654, %si",
    ));
    //
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(r15), rdx),
        "4438FA",
        "cmpb    %r15b, %dl",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(rcx), r8),
        "4138C8",
        "cmpb    %cl, %r8b",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(rcx), rsi),
        "4038CE",
        "cmpb    %cl, %sil",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::mem(Addr::imm_reg(99, rdi)), rdx),
        "3A5763",
        "cmpb    99(%rdi), %dl",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::mem(Addr::imm_reg(99, rdi)), r8),
        "443A4763",
        "cmpb    99(%rdi), %r8b",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::mem(Addr::imm_reg(99, rdi)), rsi),
        "403A7763",
        "cmpb    99(%rdi), %sil",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::imm(70), rdx),
        "80FA46",
        "cmpb    $70, %dl",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::imm(-76i32 as u32), r8),
        "4180F8B4",
        "cmpb    $-76, %r8b",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::imm(76), rsi),
        "4080FE4C",
        "cmpb    $76, %sil",
    ));
    // Extra byte-cases (paranoia!) for cmp_rmi_r for first operand = R
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(rax), rbx),
        "38C3",
        "cmpb    %al, %bl",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(rbx), rax),
        "38D8",
        "cmpb    %bl, %al",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(rcx), rdx),
        "38CA",
        "cmpb    %cl, %dl",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(rcx), rsi),
        "4038CE",
        "cmpb    %cl, %sil",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(rcx), r10),
        "4138CA",
        "cmpb    %cl, %r10b",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(rcx), r14),
        "4138CE",
        "cmpb    %cl, %r14b",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(rbp), rdx),
        "4038EA",
        "cmpb    %bpl, %dl",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(rbp), rsi),
        "4038EE",
        "cmpb    %bpl, %sil",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(rbp), r10),
        "4138EA",
        "cmpb    %bpl, %r10b",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(rbp), r14),
        "4138EE",
        "cmpb    %bpl, %r14b",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(r9), rdx),
        "4438CA",
        "cmpb    %r9b, %dl",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(r9), rsi),
        "4438CE",
        "cmpb    %r9b, %sil",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(r9), r10),
        "4538CA",
        "cmpb    %r9b, %r10b",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(r9), r14),
        "4538CE",
        "cmpb    %r9b, %r14b",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(r13), rdx),
        "4438EA",
        "cmpb    %r13b, %dl",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(r13), rsi),
        "4438EE",
        "cmpb    %r13b, %sil",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(r13), r10),
        "4538EA",
        "cmpb    %r13b, %r10b",
    ));
    insns.push((
        Inst::cmp_rmi_r(1, RegMemImm::reg(r13), r14),
        "4538EE",
        "cmpb    %r13b, %r14b",
    ));

    // ========================================================
    // Push64
    insns.push((Inst::push64(RegMemImm::reg(rdi)), "57", "pushq   %rdi"));
    insns.push((Inst::push64(RegMemImm::reg(r8)), "4150", "pushq   %r8"));
    insns.push((
        Inst::push64(RegMemImm::mem(Addr::imm_reg_reg_shift(321, rsi, rcx, 3))),
        "FFB4CE41010000",
        "pushq   321(%rsi,%rcx,8)",
    ));
    insns.push((
        Inst::push64(RegMemImm::mem(Addr::imm_reg_reg_shift(321, r9, rbx, 2))),
        "41FFB49941010000",
        "pushq   321(%r9,%rbx,4)",
    ));
    insns.push((Inst::push64(RegMemImm::imm(0)), "6A00", "pushq   $0"));
    insns.push((Inst::push64(RegMemImm::imm(127)), "6A7F", "pushq   $127"));
    insns.push((
        Inst::push64(RegMemImm::imm(128)),
        "6880000000",
        "pushq   $128",
    ));
    insns.push((
        Inst::push64(RegMemImm::imm(0x31415927)),
        "6827594131",
        "pushq   $826366247",
    ));
    insns.push((
        Inst::push64(RegMemImm::imm(-128i32 as u32)),
        "6A80",
        "pushq   $-128",
    ));
    insns.push((
        Inst::push64(RegMemImm::imm(-129i32 as u32)),
        "687FFFFFFF",
        "pushq   $-129",
    ));
    insns.push((
        Inst::push64(RegMemImm::imm(-0x75c4e8a1i32 as u32)),
        "685F173B8A",
        "pushq   $-1975838881",
    ));

    // ========================================================
    // Pop64
    insns.push((Inst::pop64(w_rax), "58", "popq    %rax"));
    insns.push((Inst::pop64(w_rdi), "5F", "popq    %rdi"));
    insns.push((Inst::pop64(w_r8), "4158", "popq    %r8"));
    insns.push((Inst::pop64(w_r15), "415F", "popq    %r15"));

    // ========================================================
    // CallKnown skipped for now

    // ========================================================
    // CallUnknown
    insns.push((
        Inst::call_unknown(RegMem::reg(rbp)),
        "FFD5",
        "call    *%rbp",
    ));
    insns.push((
        Inst::call_unknown(RegMem::reg(r11)),
        "41FFD3",
        "call    *%r11",
    ));
    insns.push((
        Inst::call_unknown(RegMem::mem(Addr::imm_reg_reg_shift(321, rsi, rcx, 3))),
        "FF94CE41010000",
        "call    *321(%rsi,%rcx,8)",
    ));
    insns.push((
        Inst::call_unknown(RegMem::mem(Addr::imm_reg_reg_shift(321, r10, rdx, 2))),
        "41FF949241010000",
        "call    *321(%r10,%rdx,4)",
    ));

    // ========================================================
    // Ret
    insns.push((Inst::ret(), "C3", "ret"));

    // ========================================================
    // JmpKnown skipped for now

    // ========================================================
    // JmpCondSymm isn't a real instruction

    // ========================================================
    // JmpCond skipped for now

    // ========================================================
    // JmpCondCompound isn't a real instruction

    // ========================================================
    // JmpUnknown
    insns.push((Inst::jmp_unknown(RegMem::reg(rbp)), "FFE5", "jmp     *%rbp"));
    insns.push((
        Inst::jmp_unknown(RegMem::reg(r11)),
        "41FFE3",
        "jmp     *%r11",
    ));
    insns.push((
        Inst::jmp_unknown(RegMem::mem(Addr::imm_reg_reg_shift(321, rsi, rcx, 3))),
        "FFA4CE41010000",
        "jmp     *321(%rsi,%rcx,8)",
    ));
    insns.push((
        Inst::jmp_unknown(RegMem::mem(Addr::imm_reg_reg_shift(321, r10, rdx, 2))),
        "41FFA49241010000",
        "jmp     *321(%r10,%rdx,4)",
    ));

    // ========================================================
    // XMM_RM_R

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Addss, RegMem::reg(xmm1), w_xmm0),
        "F30F58C1",
        "addss   %xmm1, %xmm0",
    ));
    insns.push((
        Inst::xmm_rm_r(SseOpcode::Subss, RegMem::reg(xmm0), w_xmm1),
        "F30F5CC8",
        "subss   %xmm0, %xmm1",
    ));
    insns.push((
        Inst::xmm_rm_r(SseOpcode::Addss, RegMem::reg(xmm11), w_xmm13),
        "F3450F58EB",
        "addss   %xmm11, %xmm13",
    ));
    insns.push((
        Inst::xmm_rm_r(SseOpcode::Subss, RegMem::reg(xmm12), w_xmm1),
        "F3410F5CCC",
        "subss   %xmm12, %xmm1",
    ));
    insns.push((
        Inst::xmm_rm_r(
            SseOpcode::Addss,
            RegMem::mem(Addr::imm_reg_reg_shift(123, r10, rdx, 2)),
            w_xmm0,
        ),
        "F3410F5844927B",
        "addss   123(%r10,%rdx,4), %xmm0",
    ));
    insns.push((
        Inst::xmm_rm_r(
            SseOpcode::Subss,
            RegMem::mem(Addr::imm_reg_reg_shift(321, r10, rax, 3)),
            w_xmm10,
        ),
        "F3450F5C94C241010000",
        "subss   321(%r10,%rax,8), %xmm10",
    ));
    insns.push((
        Inst::xmm_rm_r(SseOpcode::Mulss, RegMem::reg(xmm5), w_xmm4),
        "F30F59E5",
        "mulss   %xmm5, %xmm4",
    ));
    insns.push((
        Inst::xmm_rm_r(SseOpcode::Divss, RegMem::reg(xmm8), w_xmm7),
        "F3410F5EF8",
        "divss   %xmm8, %xmm7",
    ));

    insns.push((
        Inst::xmm_rm_r(SseOpcode::Sqrtss, RegMem::reg(xmm7), w_xmm8),
        "F3440F51C7",
        "sqrtss  %xmm7, %xmm8",
    ));

    // ========================================================
    // XMM_R_R

    insns.push((
        Inst::xmm_r_r(SseOpcode::Movss, xmm3, w_xmm2),
        "F30F10D3",
        "movss   %xmm3, %xmm2",
    ));

    insns.push((
        Inst::xmm_r_r(SseOpcode::Movsd, xmm4, w_xmm3),
        "F20F10DC",
        "movsd   %xmm4, %xmm3",
    ));

    // ========================================================
    // Actually run the tests!
    let flags = settings::Flags::new(settings::builder());
    let rru = regs::create_reg_universe_systemv(&flags);
    for (insn, expected_encoding, expected_printing) in insns {
        // Check the printed text is as expected.
        let actual_printing = insn.show_rru(Some(&rru));
        assert_eq!(expected_printing, actual_printing);
        let mut sink = test_utils::TestCodeSink::new();
        let mut buffer = MachBuffer::new();
        insn.emit(&mut buffer, &flags, &mut Default::default());
        let buffer = buffer.finish();
        buffer.emit(&mut sink);
        let actual_encoding = &sink.stringify();
        assert_eq!(expected_encoding, actual_encoding);
    }
}
