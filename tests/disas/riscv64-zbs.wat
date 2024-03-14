;;! target = "riscv64"
;;! test = "compile"
;;! flags = "-Ccranelift-has-zbs"

(module
  (func (export "bclr32") (param i32 i32) (result i32)
    (i32.and (local.get 0) (i32.xor (i32.shl (i32.const 1) (local.get 1)) (i32.const -1)))
  )
  (func (export "bclr64") (param i64 i64) (result i64)
    (i64.and (i64.xor (i64.shl (i64.const 1) (local.get 1)) (i64.const -1)) (local.get 0))
  )
  (func (export "bclri32_4") (param i32) (result i32)
    (i32.and (local.get 0) (i32.xor (i32.shl (i32.const 1) (i32.const 4)) (i32.const -1)))
  )
  (func (export "bclri32_20") (param i32) (result i32)
    (i32.and (i32.xor (i32.shl (i32.const 1) (i32.const 20)) (i32.const -1)) (local.get 0))
  )
  (func (export "bclri64_4") (param i64) (result i64)
    (i64.and (local.get 0) (i64.xor (i64.shl (i64.const 1) (i64.const 4)) (i64.const -1)))
  )
  (func (export "bclri64_52") (param i64) (result i64)
    (i64.and (local.get 0) (i64.xor (i64.shl (i64.const 1) (i64.const 52)) (i64.const -1)))
  )

  (func (export "bext32_1") (param i32 i32) (result i32)
    (i32.and (i32.shr_u (local.get 0) (local.get 1)) (i32.const 1))
  )
  (func (export "bext32_2") (param i32 i32) (result i32)
    (i32.and (i32.shr_s (local.get 0) (local.get 1)) (i32.const 1))
  )
  (func (export "bext32_3") (param i32 i32) (result i32)
    (i32.and (i32.const 1) (i32.shr_u (local.get 0) (local.get 1)))
  )
  (func (export "bext32_4") (param i32 i32) (result i32)
    (i32.and (i32.const 1) (i32.shr_s (local.get 0) (local.get 1)))
  )
  (func (export "bext64_1") (param i64 i64) (result i64)
    (i64.and (i64.shr_u (local.get 0) (local.get 1)) (i64.const 1))
  )
  (func (export "bext64_2") (param i64 i64) (result i64)
    (i64.and (i64.shr_s (local.get 0) (local.get 1)) (i64.const 1))
  )
  (func (export "bext64_3") (param i64 i64) (result i64)
    (i64.and (i64.const 1) (i64.shr_u (local.get 0) (local.get 1)))
  )
  (func (export "bext64_4") (param i64 i64) (result i64)
    (i64.and (i64.const 1) (i64.shr_s (local.get 0) (local.get 1)))
  )

  (func (export "bexti32_1") (param i32) (result i32)
    (i32.and (i32.shr_u (local.get 0) (i32.const 10)) (i32.const 1))
  )
  (func (export "bexti32_2") (param i32) (result i32)
    (i32.and (i32.shr_s (local.get 0) (i32.const 20)) (i32.const 1))
  )
  (func (export "bexti32_3") (param i32) (result i32)
    (i32.and (i32.shr_u (i32.const 1) (local.get 0) (i32.const 30)))
  )
  (func (export "bexti32_4") (param i32) (result i32)
    (i32.and (i32.shr_s (i32.const 1) (local.get 0) (i32.const 40)))
  )
  (func (export "bexti64_1") (param i64) (result i64)
    (i64.and (i64.shr_u (local.get 0) (i64.const 10)) (i64.const 1))
  )
  (func (export "bexti64_2") (param i64) (result i64)
    (i64.and (i64.shr_s (local.get 0) (i64.const 20)) (i64.const 1))
  )
  (func (export "bexti64_3") (param i64) (result i64)
    (i64.and (i64.shr_u (i64.const 1) (local.get 0) (i64.const 30)))
  )
  (func (export "bexti64_4") (param i64) (result i64)
    (i64.and (i64.shr_s (i64.const 1) (local.get 0) (i64.const 40)))
  )

  (func (export "binv32_1") (param i32 i32) (result i32)
    (i32.xor (local.get 0) (i32.shl (i32.const 1) (local.get 1)))
  )
  (func (export "binv32_2") (param i32 i32) (result i32)
    (i32.xor (i32.shl (i32.const 1) (local.get 1)) (local.get 0))
  )
  (func (export "binv64_1") (param i64 i64) (result i64)
    (i64.xor (local.get 0) (i64.shl (i64.const 1) (local.get 1)))
  )
  (func (export "binv64_2") (param i64 i64) (result i64)
    (i64.xor (i64.shl (i64.const 1) (local.get 1)) (local.get 0))
  )
  (func (export "binvi32_1") (param i32) (result i32)
    (i32.xor (local.get 0) (i32.shl (i32.const 1) (i32.const 10)))
  )
  (func (export "binvi32_2") (param i32) (result i32)
    (i32.xor (i32.shl (i32.const 1) (i32.const 20)) (local.get 0))
  )
  (func (export "binvi64_1") (param i64) (result i64)
    (i64.xor (local.get 0) (i64.shl (i64.const 1) (i64.const 30)))
  )
  (func (export "binvi64_2") (param i64) (result i64)
    (i64.xor (i64.shl (i64.const 1) (i64.const 40)) (local.get 0))
  )

  (func (export "bset32_1") (param i32 i32) (result i32)
    (i32.or (local.get 0) (i32.shl (i32.const 1) (local.get 1)))
  )
  (func (export "bset32_2") (param i32 i32) (result i32)
    (i32.or (i32.shl (i32.const 1) (local.get 1)) (local.get 0))
  )
  (func (export "bset64_1") (param i64 i64) (result i64)
    (i64.or (local.get 0) (i64.shl (i64.const 1) (local.get 1)))
  )
  (func (export "bset64_2") (param i64 i64) (result i64)
    (i64.or (i64.shl (i64.const 1) (local.get 1)) (local.get 0))
  )
  (func (export "bseti32_1") (param i32) (result i32)
    (i32.or (local.get 0) (i32.shl (i32.const 1) (i32.const 10)))
  )
  (func (export "bseti32_2") (param i32) (result i32)
    (i32.or (i32.shl (i32.const 1) (i32.const 20)) (local.get 0))
  )
  (func (export "bseti64_1") (param i64) (result i64)
    (i64.or (local.get 0) (i64.shl (i64.const 1) (i64.const 30)))
  )
  (func (export "bseti64_2") (param i64) (result i64)
    (i64.or (i64.shl (i64.const 1) (i64.const 40)) (local.get 0))
  )
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
;;   andi a0,a3,31
;;   bclr a0,a2,a0
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
;;   bclr a0,a2,a3
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
;;   bclri a0,a2,4
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
;;   bclri a0,a2,20
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
;;   bclri a0,a2,4
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
;;   bclri a0,a2,52
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
;;   andi a0,a3,31
;;   bext a0,a2,a0
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
;;   andi a0,a3,31
;;   bext a0,a2,a0
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
;;   andi a0,a3,31
;;   bext a0,a2,a0
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
;;   andi a0,a3,31
;;   bext a0,a2,a0
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
;;   bext a0,a2,a3
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
;;   bext a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:12:
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
;;   bext a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:13:
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
;;   bext a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:14:
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
;;   bexti a0,a2,10
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:15:
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
;;   bexti a0,a2,20
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:16:
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
;;   bexti a0,a2,30
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:17:
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
;;   bexti a0,a2,8
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:18:
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
;;   bexti a0,a2,10
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:19:
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
;;   bexti a0,a2,20
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:20:
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
;;   bexti a0,a2,30
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:21:
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
;;   bexti a0,a2,40
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:22:
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
;;   andi a0,a3,31
;;   binv a0,a2,a0
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:23:
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
;;   andi a0,a3,31
;;   binv a0,a2,a0
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:24:
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
;;   binv a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:25:
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
;;   binv a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:26:
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
;;   binvi a0,a2,10
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:27:
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
;;   binvi a0,a2,20
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:28:
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
;;   binvi a0,a2,30
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:29:
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
;;   binvi a0,a2,40
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:30:
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
;;   andi a0,a3,31
;;   bset a0,a2,a0
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:31:
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
;;   andi a0,a3,31
;;   bset a0,a2,a0
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:32:
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
;;   bset a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:33:
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
;;   bset a0,a2,a3
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:34:
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
;;   bseti a0,a2,10
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:35:
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
;;   bseti a0,a2,20
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:36:
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
;;   bseti a0,a2,30
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
;;
;; function u0:37:
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
;;   bseti a0,a2,40
;;   ld ra,8(sp)
;;   ld fp,0(sp)
;;   addi sp,sp,16
;;   ret
