;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (func (param anyref) (result i32)
    (ref.test (ref eq) (local.get 0))
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
;; @001b                               v4 = iconst.i32 0
;; @001b                               v5 = icmp eq v2, v4  ; v4 = 0
;; @001b                               brif v5, block4(v4), block2  ; v4 = 0
;;
;;                                 block2:
;; @001b                               v8 = iconst.i32 1
;; @001b                               v9 = band.i32 v2, v8  ; v8 = 1
;; @001b                               brif v9, block4(v8), block3  ; v8 = 1
;;
;;                                 block3:
;; @001b                               v22 = load.i64 notrap aligned readonly can_move v0+8
;; @001b                               v12 = load.i64 notrap aligned readonly can_move v22+32
;; @001b                               v11 = uextend.i64 v2
;; @001b                               v13 = iadd v12, v11
;; @001b                               v16 = load.i32 user2 readonly region0 v13
;; @001b                               v17 = iconst.i32 -1610612736
;; @001b                               v18 = band v16, v17  ; v17 = -1610612736
;; @001b                               v19 = icmp eq v18, v17  ; v17 = -1610612736
;; @001b                               v20 = uextend.i32 v19
;; @001b                               jump block4(v20)
;;
;;                                 block4(v21: i32):
;; @001e                               jump block1(v21)
;;
;;                                 block1(v3: i32):
;; @001e                               return v3
;; }
