;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $s (struct))
  (func (param anyref) (result i32)
    (ref.test (ref $s) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001d                               v4 = iconst.i32 0
;; @001d                               v5 = icmp eq v2, v4  ; v4 = 0
;; @001d                               v6 = uextend.i32 v5
;; @001d                               brif v6, block4(v4), block2  ; v4 = 0
;;
;;                                 block2:
;; @001d                               v8 = iconst.i32 1
;; @001d                               v9 = band.i32 v2, v8  ; v8 = 1
;;                                     v28 = iconst.i32 0
;; @001d                               brif v9, block4(v28), block3  ; v28 = 0
;;
;;                                 block3:
;; @001d                               v26 = load.i64 notrap aligned readonly can_move v0+8
;; @001d                               v15 = load.i64 notrap aligned readonly can_move v26+32
;; @001d                               v14 = uextend.i64 v2
;; @001d                               v16 = iadd v15, v14
;; @001d                               v17 = iconst.i64 4
;; @001d                               v18 = iadd v16, v17  ; v17 = 4
;; @001d                               v19 = load.i32 user2 readonly region0 v18
;; @001d                               v12 = load.i64 notrap aligned readonly can_move v0+40
;; @001d                               v13 = load.i32 notrap aligned readonly can_move v12
;; @001d                               v20 = icmp eq v19, v13
;; @001d                               v21 = uextend.i32 v20
;; @001d                               brif v21, block6(v21), block5
;;
;;                                 block5:
;; @001d                               v23 = call fn0(v0, v19, v13)
;; @001d                               jump block6(v23)
;;
;;                                 block6(v24: i32):
;; @001d                               jump block4(v24)
;;
;;                                 block4(v25: i32):
;; @0020                               jump block1(v25)
;;
;;                                 block1(v3: i32):
;; @0020                               return v3
;; }
