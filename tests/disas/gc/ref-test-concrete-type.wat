;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (type $s (struct))
  (func (param anyref) (result i32)
    (ref.test (ref $s) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     fn0 = colocated u1:35 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v34 = iconst.i32 0
;; @001d                               v4 = icmp eq v2, v34  ; v34 = 0
;; @001d                               v5 = uextend.i32 v4
;; @001d                               v7 = iconst.i32 1
;;                                     v39 = select v2, v7, v34  ; v7 = 1, v34 = 0
;; @001d                               brif v5, block4(v39), block2
;;
;;                                 block2:
;;                                     v46 = iconst.i32 1
;;                                     v47 = band.i32 v2, v46  ; v46 = 1
;;                                     v48 = iconst.i32 0
;;                                     v49 = select v47, v48, v46  ; v48 = 0, v46 = 1
;; @001d                               brif v47, block4(v49), block3
;;
;;                                 block3:
;; @001d                               v20 = uextend.i64 v2
;; @001d                               v21 = iconst.i64 4
;; @001d                               v22 = uadd_overflow_trap v20, v21, user1  ; v21 = 4
;; @001d                               v23 = iconst.i64 8
;; @001d                               v24 = uadd_overflow_trap v22, v23, user1  ; v23 = 8
;; @001d                               v19 = load.i64 notrap aligned readonly v0+48
;; @001d                               v25 = icmp ule v24, v19
;; @001d                               trapz v25, user1
;; @001d                               v18 = load.i64 notrap aligned readonly v0+40
;; @001d                               v26 = iadd v18, v22
;; @001d                               v27 = load.i32 notrap aligned readonly v26
;; @001d                               v15 = load.i64 notrap aligned readonly v0+80
;; @001d                               v16 = load.i32 notrap aligned readonly v15
;; @001d                               v28 = icmp eq v27, v16
;; @001d                               v29 = uextend.i32 v28
;; @001d                               brif v29, block6(v29), block5
;;
;;                                 block5:
;; @001d                               v31 = call fn0(v0, v27, v16)
;; @001d                               jump block6(v31)
;;
;;                                 block6(v32: i32):
;; @001d                               jump block4(v32)
;;
;;                                 block4(v33: i32):
;; @0020                               jump block1(v33)
;;
;;                                 block1(v3: i32):
;; @0020                               return v3
;; }
