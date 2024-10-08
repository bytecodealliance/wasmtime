;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (func (param anyref) (result i32)
    (ref.test (ref array) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v31 = iconst.i32 0
;; @001b                               v4 = icmp eq v2, v31  ; v31 = 0
;; @001b                               v5 = uextend.i32 v4
;; @001b                               v7 = iconst.i32 1
;;                                     v36 = select v2, v7, v31  ; v7 = 1, v31 = 0
;; @001b                               brif v5, block4(v36), block2
;;
;;                                 block2:
;;                                     v44 = iconst.i32 1
;;                                     v45 = band.i32 v2, v44  ; v44 = 1
;;                                     v46 = iconst.i32 0
;;                                     v47 = select v45, v46, v44  ; v46 = 0, v44 = 1
;; @001b                               brif v45, block4(v47), block3
;;
;;                                 block3:
;; @001b                               v18 = uextend.i64 v2
;; @001b                               v19 = iconst.i64 0
;; @001b                               v20 = uadd_overflow_trap v18, v19, user1  ; v19 = 0
;;                                     v43 = iconst.i64 8
;; @001b                               v22 = uadd_overflow_trap v18, v43, user1  ; v43 = 8
;; @001b                               v17 = load.i64 notrap aligned readonly v0+48
;; @001b                               v23 = icmp ule v22, v17
;; @001b                               trapz v23, user1
;; @001b                               v16 = load.i64 notrap aligned readonly v0+40
;; @001b                               v24 = iadd v16, v20
;; @001b                               v25 = load.i32 notrap aligned readonly v24
;; @001b                               v26 = iconst.i32 -1543503872
;; @001b                               v27 = band v25, v26  ; v26 = -1543503872
;; @001b                               v28 = icmp eq v27, v26  ; v26 = -1543503872
;; @001b                               v29 = uextend.i32 v28
;; @001b                               jump block4(v29)
;;
;;                                 block4(v30: i32):
;; @001e                               jump block1(v30)
;;
;;                                 block1(v3: i32):
;; @001e                               return v3
;; }
