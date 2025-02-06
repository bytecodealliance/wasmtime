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
;;     sig0 = (i64 vmctx, i32, i32) -> i32 tail
;;     fn0 = colocated u1:35 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v31 = iconst.i32 0
;; @001d                               v4 = icmp eq v2, v31  ; v31 = 0
;; @001d                               v5 = uextend.i32 v4
;; @001d                               brif v5, block4(v31), block2  ; v31 = 0
;;
;;                                 block2:
;; @001d                               v7 = iconst.i32 1
;; @001d                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v32 = iconst.i32 0
;; @001d                               brif v8, block4(v32), block3  ; v32 = 0
;;
;;                                 block3:
;; @001d                               v17 = uextend.i64 v2
;; @001d                               v18 = iconst.i64 4
;; @001d                               v19 = uadd_overflow_trap v17, v18, user1  ; v18 = 4
;; @001d                               v20 = iconst.i64 8
;; @001d                               v21 = uadd_overflow_trap v19, v20, user1  ; v20 = 8
;; @001d                               v16 = load.i64 notrap aligned readonly v0+48
;; @001d                               v22 = icmp ule v21, v16
;; @001d                               trapz v22, user1
;; @001d                               v14 = load.i64 notrap aligned readonly v0+40
;; @001d                               v23 = iadd v14, v19
;; @001d                               v24 = load.i32 notrap aligned readonly v23
;; @001d                               v11 = load.i64 notrap aligned readonly v0+64
;; @001d                               v12 = load.i32 notrap aligned readonly v11
;; @001d                               v25 = icmp eq v24, v12
;; @001d                               v26 = uextend.i32 v25
;; @001d                               brif v26, block6(v26), block5
;;
;;                                 block5:
;; @001d                               v28 = call fn0(v0, v24, v12)
;; @001d                               jump block6(v28)
;;
;;                                 block6(v29: i32):
;; @001d                               jump block4(v29)
;;
;;                                 block4(v30: i32):
;; @0020                               jump block1(v30)
;;
;;                                 block1(v3: i32):
;; @0020                               return v3
;; }
