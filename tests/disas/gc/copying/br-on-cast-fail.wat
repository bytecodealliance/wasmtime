;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $s (struct))
  (import "" "f" (func $f))
  (import "" "g" (func $g))
  (func (param anyref)
    block (result anyref)
      (br_on_cast_fail 0 anyref (ref $s) (local.get 0))
      (call $f)
      return
    end
    (call $g)
    return
  )
)
;; function u0:0(i64 vmctx, i64, i32) tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) tail
;;     fn0 = colocated u805306368:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v42 = stack_addr.i64 ss0
;;                                     store notrap v2, v42
;; @002e                               v4 = iconst.i32 0
;; @002e                               v5 = icmp eq v2, v4  ; v4 = 0
;; @002e                               brif v5, block5(v4), block3  ; v4 = 0
;;
;;                                 block3:
;; @002e                               v8 = iconst.i32 1
;; @002e                               v9 = band.i32 v2, v8  ; v8 = 1
;;                                     v43 = iconst.i32 0
;; @002e                               brif v9, block5(v43), block4  ; v43 = 0
;;
;;                                 block4:
;; @002e                               v37 = load.i64 notrap aligned readonly can_move v0+8
;; @002e                               v15 = load.i64 notrap aligned readonly can_move v37+32
;; @002e                               v14 = uextend.i64 v2
;; @002e                               v16 = iadd v15, v14
;; @002e                               v17 = iconst.i64 4
;; @002e                               v18 = iadd v16, v17  ; v17 = 4
;; @002e                               v19 = load.i32 user2 readonly region0 v18
;; @002e                               v12 = load.i64 notrap aligned readonly can_move v0+40
;; @002e                               v13 = load.i32 notrap aligned readonly can_move v12
;; @002e                               v20 = icmp eq v19, v13
;; @002e                               v21 = uextend.i32 v20
;; @002e                               brif v20, block7(v21), block6
;;
;;                                 block6:
;; @002e                               v23 = call fn0(v0, v19, v13), stack_map=[i32 @ ss0+0]
;; @002e                               jump block7(v23)
;;
;;                                 block7(v24: i32):
;; @002e                               jump block5(v24)
;;
;;                                 block5(v25: i32):
;;                                     v32 = load.i32 notrap v42
;; @002e                               brif v25, block8, block2
;;
;;                                 block8:
;; @0034                               v28 = load.i64 notrap aligned readonly can_move v0+56
;; @0034                               v27 = load.i64 notrap aligned readonly can_move v0+72
;; @0034                               call_indirect sig1, v28(v27, v0)
;; @0036                               return
;;
;;                                 block2:
;; @0038                               v31 = load.i64 notrap aligned readonly can_move v0+88
;; @0038                               v30 = load.i64 notrap aligned readonly can_move v0+104
;; @0038                               call_indirect sig1, v31(v30, v0)
;; @003a                               return
;; }
