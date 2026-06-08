;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $s (struct))
  (import "" "f" (func $f))
  (import "" "g" (func $g))
  (func (param anyref)
    block (result (ref $s))
      (br_on_cast 0 anyref (ref $s) (local.get 0))
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
;; @002f                               v4 = iconst.i32 0
;; @002f                               v5 = icmp eq v2, v4  ; v4 = 0
;; @002f                               brif v5, block5(v4), block3  ; v4 = 0
;;
;;                                 block3:
;; @002f                               v8 = iconst.i32 1
;; @002f                               v9 = band.i32 v2, v8  ; v8 = 1
;;                                     v30 = iconst.i32 0
;; @002f                               brif v9, block5(v30), block4  ; v30 = 0
;;
;;                                 block4:
;; @002f                               v28 = load.i64 notrap aligned readonly can_move v0+8
;; @002f                               v14 = load.i64 notrap aligned readonly can_move v28+32
;; @002f                               v13 = uextend.i64 v2
;; @002f                               v15 = iadd v14, v13
;; @002f                               v16 = iconst.i64 4
;; @002f                               v17 = iadd v15, v16  ; v16 = 4
;; @002f                               v18 = load.i32 user2 readonly region0 v17
;; @002f                               v11 = load.i64 notrap aligned readonly can_move v0+40
;; @002f                               v12 = load.i32 notrap aligned readonly can_move v11
;; @002f                               v19 = icmp eq v18, v12
;; @002f                               v20 = uextend.i32 v19
;; @002f                               jump block5(v20)
;;
;;                                 block5(v21: i32):
;; @002f                               brif v21, block2, block6
;;
;;                                 block6:
;; @0035                               v24 = load.i64 notrap aligned readonly can_move v0+56
;; @0035                               v23 = load.i64 notrap aligned readonly can_move v0+72
;; @0035                               call_indirect sig0, v24(v23, v0)
;; @0037                               return
;;
;;                                 block2:
;; @0039                               v27 = load.i64 notrap aligned readonly can_move v0+88
;; @0039                               v26 = load.i64 notrap aligned readonly can_move v0+104
;; @0039                               call_indirect sig0, v27(v26, v0)
;; @003b                               return
;; }
