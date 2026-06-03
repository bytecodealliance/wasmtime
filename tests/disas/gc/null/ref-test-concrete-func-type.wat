;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $f (func (param i32) (result i32)))
  (func (param funcref) (result i32)
    (ref.test (ref $f) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i64) -> i32 tail {
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v4 = iconst.i64 0
;; @0020                               v5 = icmp eq v2, v4  ; v4 = 0
;; @0020                               v7 = iconst.i32 0
;; @0020                               brif v5, block4(v7), block2  ; v7 = 0
;;
;;                                 block2:
;; @0020                               jump block3
;;
;;                                 block3:
;; @0020                               v10 = load.i32 user2 readonly region0 v2+16
;; @0020                               v8 = load.i64 notrap aligned readonly can_move v0+40
;; @0020                               v9 = load.i32 notrap aligned readonly can_move v8
;; @0020                               v11 = icmp eq v10, v9
;; @0020                               v12 = uextend.i32 v11
;; @0020                               jump block4(v12)
;;
;;                                 block4(v13: i32):
;; @0023                               jump block1(v13)
;;
;;                                 block1(v3: i32):
;; @0023                               return v3
;; }
