;;! target = "riscv64"
;;! test = "compile"
;;! flags = "-Ccranelift-has-zbb"

(module
  (func (export "rolw") (param i32 i32) (result i32)
    (i32.rotl (local.get 0) (local.get 1)))
  (func (export "rol") (param i64 i64) (result i64)
    (i64.rotl (local.get 0) (local.get 1)))
  (func (export "rolwi") (param i32 ) (result i32)
    (i32.rotl (local.get 0) (i32.const 100)))
  (func (export "roli") (param i64) (result i64)
    (i64.rotl (local.get 0) (i64.const 40)))

  (func (export "rorw") (param i32 i32) (result i32)
    (i32.rotr (local.get 0) (local.get 1)))
  (func (export "ror") (param i64 i64) (result i64)
    (i64.rotr (local.get 0) (local.get 1)))
  (func (export "rorwi") (param i32 ) (result i32)
    (i32.rotr (local.get 0) (i32.const 100)))
  (func (export "rori") (param i64) (result i64)
    (i64.rotr (local.get 0) (i64.const 40)))

  (func (export "xnor32_1") (param i32 i32) (result i32)
    (i32.xor (i32.xor (local.get 0) (local.get 1)) (i32.const -1)))
  (func (export "xnor32_2") (param i32 i32) (result i32)
    (i32.xor (i32.const -1) (i32.xor (local.get 0) (local.get 1))))
  (func (export "xnor64_1") (param i64 i64) (result i64)
    (i64.xor (i64.xor (local.get 0) (local.get 1)) (i64.const -1)))
  (func (export "xnor64_2") (param i64 i64) (result i64)
    (i64.xor (i64.const -1) (i64.xor (local.get 0) (local.get 1))))
)
;; function u0:0:
;;   addi sp,sp,-16
;;   sd ra,8(sp)
;;   sd fp,0(sp)
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mv fp,sp
;;   ld t6,8(a0)
;;   ld t6,0(t6)
;;   trap_if stk_ovf##(sp ult t6)
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   j label1
;; block1:
;;   rolw a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:1:
;;   addi sp,sp,-16
;;   sd ra,8(sp)
;;   sd fp,0(sp)
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mv fp,sp
;;   ld t6,8(a0)
;;   ld t6,0(t6)
;;   trap_if stk_ovf##(sp ult t6)
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   j label1
;; block1:
;;   rol a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:2:
;;   addi sp,sp,-16
;;   sd ra,8(sp)
;;   sd fp,0(sp)
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mv fp,sp
;;   ld t6,8(a0)
;;   ld t6,0(t6)
;;   trap_if stk_ovf##(sp ult t6)
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   j label1
;; block1:
;;   roriw a0,a2,28
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:3:
;;   addi sp,sp,-16
;;   sd ra,8(sp)
;;   sd fp,0(sp)
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mv fp,sp
;;   ld t6,8(a0)
;;   ld t6,0(t6)
;;   trap_if stk_ovf##(sp ult t6)
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   j label1
;; block1:
;;   rori a0,a2,24
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:4:
;;   addi sp,sp,-16
;;   sd ra,8(sp)
;;   sd fp,0(sp)
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mv fp,sp
;;   ld t6,8(a0)
;;   ld t6,0(t6)
;;   trap_if stk_ovf##(sp ult t6)
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   j label1
;; block1:
;;   rorw a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:5:
;;   addi sp,sp,-16
;;   sd ra,8(sp)
;;   sd fp,0(sp)
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mv fp,sp
;;   ld t6,8(a0)
;;   ld t6,0(t6)
;;   trap_if stk_ovf##(sp ult t6)
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   j label1
;; block1:
;;   ror a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:6:
;;   addi sp,sp,-16
;;   sd ra,8(sp)
;;   sd fp,0(sp)
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mv fp,sp
;;   ld t6,8(a0)
;;   ld t6,0(t6)
;;   trap_if stk_ovf##(sp ult t6)
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   j label1
;; block1:
;;   roriw a0,a2,4
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:7:
;;   addi sp,sp,-16
;;   sd ra,8(sp)
;;   sd fp,0(sp)
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mv fp,sp
;;   ld t6,8(a0)
;;   ld t6,0(t6)
;;   trap_if stk_ovf##(sp ult t6)
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   j label1
;; block1:
;;   rori a0,a2,40
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:8:
;;   addi sp,sp,-16
;;   sd ra,8(sp)
;;   sd fp,0(sp)
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mv fp,sp
;;   ld t6,8(a0)
;;   ld t6,0(t6)
;;   trap_if stk_ovf##(sp ult t6)
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   j label1
;; block1:
;;   xnor a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:9:
;;   addi sp,sp,-16
;;   sd ra,8(sp)
;;   sd fp,0(sp)
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mv fp,sp
;;   ld t6,8(a0)
;;   ld t6,0(t6)
;;   trap_if stk_ovf##(sp ult t6)
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   j label1
;; block1:
;;   xnor a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:10:
;;   addi sp,sp,-16
;;   sd ra,8(sp)
;;   sd fp,0(sp)
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mv fp,sp
;;   ld t6,8(a0)
;;   ld t6,0(t6)
;;   trap_if stk_ovf##(sp ult t6)
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   j label1
;; block1:
;;   xnor a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:11:
;;   addi sp,sp,-16
;;   sd ra,8(sp)
;;   sd fp,0(sp)
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   mv fp,sp
;;   ld t6,8(a0)
;;   ld t6,0(t6)
;;   trap_if stk_ovf##(sp ult t6)
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   j label1
;; block1:
;;   xnor a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
