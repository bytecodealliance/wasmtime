;;! target = "riscv64"
;;! test = "compile"
;;! objdump = "--filter array_to_wasm --funcs all"

(module (func (export "")))

;; wasm[0]::array_to_wasm_trampoline[0]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       addi    sp, sp, -0xd0
;;       sd      s0, 0xc8(sp)
;;       sd      s1, 0xc0(sp)
;;       sd      s2, 0xb8(sp)
;;       sd      s3, 0xb0(sp)
;;       sd      s4, 0xa8(sp)
;;       sd      s5, 0xa0(sp)
;;       sd      s6, 0x98(sp)
;;       sd      s7, 0x90(sp)
;;       sd      s8, 0x88(sp)
;;       sd      s9, 0x80(sp)
;;       sd      s10, 0x78(sp)
;;       sd      s11, 0x70(sp)
;;       fsd     fs0, 0x68(sp)
;;       fsd     fs2, 0x60(sp)
;;       fsd     fs3, 0x58(sp)
;;       fsd     fs4, 0x50(sp)
;;       fsd     fs5, 0x48(sp)
;;       fsd     fs6, 0x40(sp)
;;       fsd     fs7, 0x38(sp)
;;       fsd     fs8, 0x30(sp)
;;       fsd     fs9, 0x28(sp)
;;       fsd     fs10, 0x20(sp)
;;       fsd     fs11, 0x18(sp)
;;       mv      a2, s0
;;       ld      a5, 8(a0)
;;       sd      a2, 0x48(a5)
;;       mv      a3, sp
;;       sd      a3, 0x40(a5)
;;       auipc   a4, 0
;;       addi    a4, a4, 0x8c
;;       sd      a4, 0x50(a5)
;;       sd      a5, 0(sp)
;;       auipc   ra, 0
;;       jalr    ra, ra, -0xb4
;;       ├─╼ exception frame offset: SP = FP - 0xd0
;;       ╰─╼ exception handler: default handler, no dynamic context, handler=0x130
;;       addi    a0, zero, 1
;;       ld      s0, 0xc8(sp)
;;       ld      s1, 0xc0(sp)
;;       ld      s2, 0xb8(sp)
;;       ld      s3, 0xb0(sp)
;;       ld      s4, 0xa8(sp)
;;       ld      s5, 0xa0(sp)
;;       ld      s6, 0x98(sp)
;;       ld      s7, 0x90(sp)
;;       ld      s8, 0x88(sp)
;;       ld      s9, 0x80(sp)
;;       ld      s10, 0x78(sp)
;;       ld      s11, 0x70(sp)
;;       fld     fs0, 0x68(sp)
;;       fld     fs2, 0x60(sp)
;;       fld     fs3, 0x58(sp)
;;       fld     fs4, 0x50(sp)
;;       fld     fs5, 0x48(sp)
;;       fld     fs6, 0x40(sp)
;;       fld     fs7, 0x38(sp)
;;       fld     fs8, 0x30(sp)
;;       fld     fs9, 0x28(sp)
;;       fld     fs10, 0x20(sp)
;;       fld     fs11, 0x18(sp)
;;       addi    sp, sp, 0xd0
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;       addi    a0, zero, 1
;;       ld      a5, 0(sp)
;;       sd      a0, 0x88(a5)
;;       mv      a0, zero
;;       ld      s0, 0xc8(sp)
;;       ld      s1, 0xc0(sp)
;;       ld      s2, 0xb8(sp)
;;       ld      s3, 0xb0(sp)
;;       ld      s4, 0xa8(sp)
;;       ld      s5, 0xa0(sp)
;;       ld      s6, 0x98(sp)
;;       ld      s7, 0x90(sp)
;;       ld      s8, 0x88(sp)
;;       ld      s9, 0x80(sp)
;;       ld      s10, 0x78(sp)
;;       ld      s11, 0x70(sp)
;;       fld     fs0, 0x68(sp)
;;       fld     fs2, 0x60(sp)
;;       fld     fs3, 0x58(sp)
;;       fld     fs4, 0x50(sp)
;;       fld     fs5, 0x48(sp)
;;       fld     fs6, 0x40(sp)
;;       fld     fs7, 0x38(sp)
;;       fld     fs8, 0x30(sp)
;;       fld     fs9, 0x28(sp)
;;       fld     fs10, 0x20(sp)
;;       fld     fs11, 0x18(sp)
;;       addi    sp, sp, 0xd0
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
