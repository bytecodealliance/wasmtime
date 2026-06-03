;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001d                               v4 = iconst.i32 0
;; @001d                               v5 = icmp eq v2, v4  ; v4 = 0
;; @001d                               brif v5, block4(v4), block2  ; v4 = 0
;;
;;                                 block2:
;; @001d                               v8 = iconst.i32 1
;; @001d                               v9 = band.i32 v2, v8  ; v8 = 1
;;                                     v24 = iconst.i32 0
;; @001d                               brif v9, block4(v24), block3  ; v24 = 0
;;
;;                                 block3:
;; @001d                               v22 = load.i64 notrap aligned readonly can_move v0+8
;; @001d                               v14 = load.i64 notrap aligned readonly can_move v22+32
;; @001d                               v13 = uextend.i64 v2
;; @001d                               v15 = iadd v14, v13
;; @001d                               v16 = iconst.i64 4
;; @001d                               v17 = iadd v15, v16  ; v16 = 4
;; @001d                               v18 = load.i32 user2 readonly region0 v17
;; @001d                               v11 = load.i64 notrap aligned readonly can_move v0+40
;; @001d                               v12 = load.i32 notrap aligned readonly can_move v11
;; @001d                               v19 = icmp eq v18, v12
;; @001d                               v20 = uextend.i32 v19
;; @001d                               jump block4(v20)
;;
;;                                 block4(v21: i32):
;; @0020                               jump block1(v21)
;;
;;                                 block1(v3: i32):
;; @0020                               return v3
;; }
