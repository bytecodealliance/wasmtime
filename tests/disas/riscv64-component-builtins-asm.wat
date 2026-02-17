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
;;       sd      s4, 8(sp)
;;       mv      a3, a2
;;       mv      s4, a1
;;       ld      a1, 0x10(a0)
;;       mv      a2, s0
;;       sd      a2, 0x30(a1)
;;       ld      a2, 8(s0)
;;       sd      a2, 0x38(a1)
;;       lw      a1, 0x20(a0)
;;       andi    a1, a1, 1
;;       bnez    a1, 8
;;       .byte   0x00, 0x00, 0x00, 0x00
;;       ╰─╼ trap: CannotLeaveComponent
;;       ld      a1, 8(a0)
;;       ld      a5, 0x10(a1)
;;       mv      a4, zero
;;       slli    a1, a4, 0x20
;;       srai    a1, a1, 0x20
;;       slli    a2, a4, 0x20
;;       srai    a2, a2, 0x20
;;       slli    a3, a3, 0x20
;;       srai    a3, a3, 0x20
;;       jalr    a5
;;       addi    a1, zero, -1
;;       beq     a0, a1, 0x1c
;;       ld      s4, 8(sp)
;;       addi    sp, sp, 0x10
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;       mv      a1, s4
;;       ld      a0, 0x10(a1)
;;       ld      a2, 0x198(a0)
;;       mv      a0, a1
;;       jalr    a2
;;       .byte   0x00, 0x00, 0x00, 0x00
