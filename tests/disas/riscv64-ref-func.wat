;;! target = "riscv64"
;;! test = "compile"

;; `ref.func` of a defined function initializes its `VMFuncRef` within the
;; `vmctx` inline, using relocations for the function's code pointers.

(module
  (elem declare func $f)
  (func $f (result i32) i32.const 42)
  (func (export "ref-func") (result funcref)
    (ref.func $f)))
;; wasm[0]::function[0]::f:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       addi    a0, zero, 0x2a
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
;;
;; wasm[0]::function[1]:
;;       addi    sp, sp, -0x10
;;       sd      ra, 8(sp)
;;       sd      s0, 0(sp)
;;       mv      s0, sp
;;       auipc   a1, 0
;;       addi    a1, a1, 0x3c
;;       sd      a1, 0x50(a0)
;;       auipc   a1, 0
;;       addi    a1, a1, -0x40
;;       sd      a1, 0x58(a0)
;;       sd      a0, 0x68(a0)
;;       ld      a1, 0x28(a0)
;;       lw      a1, 0(a1)
;;       sw      a1, 0x60(a0)
;;       addi    a0, a0, 0x50
;;       ld      ra, 8(sp)
;;       ld      s0, 0(sp)
;;       addi    sp, sp, 0x10
;;       ret
