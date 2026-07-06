;;! target = "x86_64"

;; A `br_if` to a block whose only reachable predecessor is that `br_if` (the
;; fall-through path ends in `unreachable`) does not need block parameters for
;; the branched-with values.

(module
  (import "" "f" (func $f (result i32)))
  (import "" "g" (func $g (param i32 i32)))
  (import "" "h" (func $h (param i32 i32)))
  (func
    block (result i32 i32)
      i32.const 1
      i32.const 2
      call $f
      br_if 0
      call $g
      unreachable
    end
    call $h
  )
)
;; function u0:0(i64 vmctx, i64) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1207959576 "VMFunctionImport+0x18"
;;     region3 = 1207959560 "VMFunctionImport+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i64, i32, i32) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0039                               v2 = iconst.i32 1
;; @003b                               v3 = iconst.i32 2
;; @003d                               v4 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @003d                               v5 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @003d                               v6 = call_indirect sig0, v5(v4, v0)
;; @003f                               brif v6, block2, block3
;;
;;                                 block3:
;; @0041                               v7 = load.i64 notrap aligned readonly can_move region2 v0+104
;; @0041                               v8 = load.i64 notrap aligned readonly can_move region3 v0+88
;; @0041                               call_indirect sig1, v8(v7, v0, v2, v3)  ; v2 = 1, v3 = 2
;; @0043                               trap user12
;;
;;                                 block2:
;; @0045                               v9 = load.i64 notrap aligned readonly can_move region2 v0+136
;; @0045                               v10 = load.i64 notrap aligned readonly can_move region3 v0+120
;; @0045                               call_indirect sig1, v10(v9, v0, v2, v3)  ; v2 = 1, v3 = 2
;; @0047                               jump block1
;;
;;                                 block1:
;; @0047                               return
;; }
