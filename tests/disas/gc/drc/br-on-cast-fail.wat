;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002e                               v4 = iconst.i32 0
;; @002e                               v5 = icmp eq v2, v4  ; v4 = 0
;; @002e                               brif v5, block5(v4), block3  ; v4 = 0
;;
;;                                 block3:
;; @002e                               v8 = iconst.i32 1
;; @002e                               v9 = band.i32 v2, v8  ; v8 = 1
;;                                     v30 = iconst.i32 0
;; @002e                               brif v9, block5(v30), block4  ; v30 = 0
;;
;;                                 block4:
;; @002e                               v28 = load.i64 notrap aligned readonly can_move v0+8
;; @002e                               v14 = load.i64 notrap aligned readonly can_move v28+32
;; @002e                               v13 = uextend.i64 v2
;; @002e                               v15 = iadd v14, v13
;; @002e                               v16 = iconst.i64 4
;; @002e                               v17 = iadd v15, v16  ; v16 = 4
;; @002e                               v18 = load.i32 user2 readonly region0 v17
;; @002e                               v11 = load.i64 notrap aligned readonly can_move v0+40
;; @002e                               v12 = load.i32 notrap aligned readonly can_move v11
;; @002e                               v19 = icmp eq v18, v12
;; @002e                               v20 = uextend.i32 v19
;; @002e                               jump block5(v20)
;;
;;                                 block5(v21: i32):
;; @002e                               brif v21, block6, block2
;;
;;                                 block6:
;; @0034                               v24 = load.i64 notrap aligned readonly can_move v0+56
;; @0034                               v23 = load.i64 notrap aligned readonly can_move v0+72
;; @0034                               call_indirect sig0, v24(v23, v0)
;; @0036                               return
;;
;;                                 block2:
;; @0038                               v27 = load.i64 notrap aligned readonly can_move v0+88
;; @0038                               v26 = load.i64 notrap aligned readonly can_move v0+104
;; @0038                               call_indirect sig0, v27(v26, v0)
;; @003a                               return
;; }
