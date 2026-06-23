;;! target = "x86_64"

;; The control-flow join after an if/else diamond has two predecessors, but when
;; both the consequent and the alternative pass the same values, no block
;; parameter is needed.

(module
  (import "" "f" (func $f))
  (import "" "g" (func $g))
  (func (param i32 i32 i32) (result i32 i32)
    local.get 0
    if (result i32 i32)
      call $f
      local.get 1
      local.get 2
    else
      call $g
      local.get 1
      local.get 2
    end
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) -> i32, i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 72 "VMContext+0x48"
;;     region3 = 56 "VMContext+0x38"
;;     region4 = 104 "VMContext+0x68"
;;     region5 = 88 "VMContext+0x58"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0033                               brif v2, block2, block4
;;
;;                                 block2:
;; @0035                               v5 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @0035                               v6 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @0035                               call_indirect sig0, v6(v5, v0)
;; @003b                               jump block3
;;
;;                                 block4:
;; @003c                               v7 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @003c                               v8 = load.i64 notrap aligned readonly can_move region5 v0+88
;; @003c                               call_indirect sig0, v8(v7, v0)
;; @0042                               jump block3
;;
;;                                 block3:
;; @0043                               jump block1
;;
;;                                 block1:
;; @0043                               return v3, v4
;; }
