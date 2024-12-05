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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext tail
;;     fn0 = colocated u1:35 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v35 = iconst.i32 0
;; @001d                               v4 = icmp eq v2, v35  ; v35 = 0
;; @001d                               v5 = uextend.i32 v4
;; @001d                               v7 = iconst.i32 1
;;                                     v40 = select v2, v7, v35  ; v7 = 1, v35 = 0
;; @001d                               brif v5, block4(v40), block2
;;
;;                                 block2:
;;                                     v47 = iconst.i32 1
;;                                     v48 = band.i32 v2, v47  ; v47 = 1
;;                                     v49 = iconst.i32 0
;;                                     v50 = select v48, v49, v47  ; v49 = 0, v47 = 1
;; @001d                               brif v48, block4(v50), block3
;;
;;                                 block3:
;; @001d                               v21 = uextend.i64 v2
;; @001d                               v22 = iconst.i64 4
;; @001d                               v23 = uadd_overflow_trap v21, v22, user1  ; v22 = 4
;; @001d                               v24 = iconst.i64 8
;; @001d                               v25 = uadd_overflow_trap v23, v24, user1  ; v24 = 8
;; @001d                               v20 = load.i64 notrap aligned readonly v0+48
;; @001d                               v26 = icmp ule v25, v20
;; @001d                               trapz v26, user1
;; @001d                               v18 = load.i64 notrap aligned readonly v0+40
;; @001d                               v27 = iadd v18, v23
;; @001d                               v28 = load.i32 notrap aligned readonly v27
;; @001d                               v15 = load.i64 notrap aligned readonly v0+80
;; @001d                               v16 = load.i32 notrap aligned readonly v15
;; @001d                               v29 = icmp eq v28, v16
;; @001d                               v30 = uextend.i32 v29
;; @001d                               brif v30, block6(v30), block5
;;
;;                                 block5:
;; @001d                               v32 = call fn0(v0, v28, v16)
;; @001d                               jump block6(v32)
;;
;;                                 block6(v33: i32):
;; @001d                               jump block4(v33)
;;
;;                                 block4(v34: i32):
;; @0020                               jump block1(v34)
;;
;;                                 block1(v3: i32):
;; @0020                               return v3
;; }
