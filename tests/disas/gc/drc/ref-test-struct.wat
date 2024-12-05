;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (func (param anyref) (result i32)
    (ref.test (ref struct) (local.get 0))
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
;;                                     v32 = iconst.i32 0
;; @001b                               v4 = icmp eq v2, v32  ; v32 = 0
;; @001b                               v5 = uextend.i32 v4
;; @001b                               v7 = iconst.i32 1
;;                                     v37 = select v2, v7, v32  ; v7 = 1, v32 = 0
;; @001b                               brif v5, block4(v37), block2
;;
;;                                 block2:
;;                                     v45 = iconst.i32 1
;;                                     v46 = band.i32 v2, v45  ; v45 = 1
;;                                     v47 = iconst.i32 0
;;                                     v48 = select v46, v47, v45  ; v47 = 0, v45 = 1
;; @001b                               brif v46, block4(v48), block3
;;
;;                                 block3:
;; @001b                               v19 = uextend.i64 v2
;; @001b                               v20 = iconst.i64 0
;; @001b                               v21 = uadd_overflow_trap v19, v20, user1  ; v20 = 0
;;                                     v44 = iconst.i64 8
;; @001b                               v23 = uadd_overflow_trap v19, v44, user1  ; v44 = 8
;; @001b                               v18 = load.i64 notrap aligned readonly v0+48
;; @001b                               v24 = icmp ule v23, v18
;; @001b                               trapz v24, user1
;; @001b                               v16 = load.i64 notrap aligned readonly v0+40
;; @001b                               v25 = iadd v16, v21
;; @001b                               v26 = load.i32 notrap aligned readonly v25
;; @001b                               v27 = iconst.i32 -1342177280
;; @001b                               v28 = band v26, v27  ; v27 = -1342177280
;; @001b                               v29 = icmp eq v28, v27  ; v27 = -1342177280
;; @001b                               v30 = uextend.i32 v29
;; @001b                               jump block4(v30)
;;
;;                                 block4(v31: i32):
;; @001e                               jump block1(v31)
;;
;;                                 block1(v3: i32):
;; @001e                               return v3
;; }
