;;! target = "riscv64"
;;! test = 'compile'
;;! filter = 'wasm-call'
;;! objdump = '--funcs all'

(component
  (type $a (resource (rep i32)))
  (core func $f (canon resource.drop $a))

  (core module $m (import "" "" (func (param i32))))
  (core instance (instantiate $m (with "" (instance (export "" (func $f))))))
)

;; component-trampolines[0]-wasm-call-component-resource-drop[0]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       addi    sp, sp, -0x10
;;       sd      s5, 8(sp)
;;       sd      s9, 0(sp)
;;       mv      s5, a1
;;       mv      s9, a2
;;       mv      a3, s0
;;       ld      a1, 8(a1)
;;       sd      a3, 0x30(a1)
;;       ld      a2, 8(s0)
;;       sd      a2, 0x38(a1)
;;       lw      a1, 0x20(a0)
;;       andi    a1, a1, 1
;;       sext.w  a1, a1
;;       bnez    a1, 8
;;       .byte   0x00, 0x00, 0x00, 0x00
;;       ╰─╼ trap: Normal(CannotLeaveComponent)
;;       ld      a1, 8(a0)
;;       ld      a5, 0x10(a1)
;;       mv      a4, zero
;;       slli    a1, a4, 0x20
;;       srai    a1, a1, 0x20
;;       slli    a2, a4, 0x20
;;       srai    a2, a2, 0x20
;;       mv      a3, s9
;;       slli    a3, a3, 0x20
;;       srai    a3, a3, 0x20
;;       jalr    a5
;;       addi    a1, zero, -1
;;       beq     a0, a1, 0x20
;;       ld      s5, 8(sp)
;;       ld      s9, 0(sp)
;;       addi    sp, sp, 0x10
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;       mv      a1, s5
;;       ld      a0, 0x10(a1)
;;       ld      a2, 0x190(a0)
;;       mv      a0, a1
;;       jalr    a2
;;       .byte   0x00, 0x00, 0x00, 0x00
