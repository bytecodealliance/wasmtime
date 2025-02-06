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
;;                                     v28 = iconst.i32 0
;; @001b                               v4 = icmp eq v2, v28  ; v28 = 0
;; @001b                               v5 = uextend.i32 v4
;; @001b                               brif v5, block4(v28), block2  ; v28 = 0
;;
;;                                 block2:
;; @001b                               v7 = iconst.i32 1
;; @001b                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v30 = iconst.i32 0
;; @001b                               brif v8, block4(v30), block3  ; v30 = 0
;;
;;                                 block3:
;; @001b                               v15 = uextend.i64 v2
;; @001b                               v16 = iconst.i64 0
;; @001b                               v17 = uadd_overflow_trap v15, v16, user1  ; v16 = 0
;;                                     v29 = iconst.i64 8
;; @001b                               v19 = uadd_overflow_trap v15, v29, user1  ; v29 = 8
;; @001b                               v14 = load.i64 notrap aligned readonly v0+48
;; @001b                               v20 = icmp ule v19, v14
;; @001b                               trapz v20, user1
;; @001b                               v12 = load.i64 notrap aligned readonly v0+40
;; @001b                               v21 = iadd v12, v17
;; @001b                               v22 = load.i32 notrap aligned readonly v21
;; @001b                               v23 = iconst.i32 -1342177280
;; @001b                               v24 = band v22, v23  ; v23 = -1342177280
;; @001b                               v25 = icmp eq v24, v23  ; v23 = -1342177280
;; @001b                               v26 = uextend.i32 v25
;; @001b                               jump block4(v26)
;;
;;                                 block4(v27: i32):
;; @001e                               jump block1(v27)
;;
;;                                 block1(v3: i32):
;; @001e                               return v3
;; }
