;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (func (param anyref) (result i32)
    (ref.test (ref eq) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v29 = iconst.i32 0
;; @001b                               v4 = icmp eq v2, v29  ; v29 = 0
;; @001b                               v5 = uextend.i32 v4
;; @001b                               v7 = iconst.i32 1
;;                                     v34 = select v2, v7, v29  ; v7 = 1, v29 = 0
;; @001b                               brif v5, block4(v34), block2
;;
;;                                 block2:
;;                                     v42 = iconst.i32 1
;;                                     v43 = band.i32 v2, v42  ; v42 = 1
;; @001b                               brif v43, block4(v43), block3
;;
;;                                 block3:
;; @001b                               v16 = uextend.i64 v2
;; @001b                               v17 = iconst.i64 0
;; @001b                               v18 = uadd_overflow_trap v16, v17, user1  ; v17 = 0
;;                                     v41 = iconst.i64 8
;; @001b                               v20 = uadd_overflow_trap v16, v41, user1  ; v41 = 8
;; @001b                               v15 = load.i64 notrap aligned readonly v0+48
;; @001b                               v21 = icmp ule v20, v15
;; @001b                               trapz v21, user1
;; @001b                               v13 = load.i64 notrap aligned readonly v0+40
;; @001b                               v22 = iadd v13, v18
;; @001b                               v23 = load.i32 notrap aligned readonly v22
;; @001b                               v24 = iconst.i32 -1610612736
;; @001b                               v25 = band v23, v24  ; v24 = -1610612736
;; @001b                               v26 = icmp eq v25, v24  ; v24 = -1610612736
;; @001b                               v27 = uextend.i32 v26
;; @001b                               jump block4(v27)
;;
;;                                 block4(v28: i32):
;; @001e                               jump block1(v28)
;;
;;                                 block1(v3: i32):
;; @001e                               return v3
;; }
