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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     sig0 = (i64 vmctx, i32, i32) -> i32 tail
;;     fn0 = colocated u1:36 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v27 = iconst.i32 0
;; @001d                               v4 = icmp eq v2, v27  ; v27 = 0
;; @001d                               v5 = uextend.i32 v4
;; @001d                               brif v5, block4(v27), block2  ; v27 = 0
;;
;;                                 block2:
;; @001d                               v7 = iconst.i32 1
;; @001d                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v28 = iconst.i32 0
;; @001d                               brif v8, block4(v28), block3  ; v28 = 0
;;
;;                                 block3:
;; @001d                               v25 = load.i64 notrap aligned readonly can_move v0+8
;; @001d                               v14 = load.i64 notrap aligned readonly can_move v25+24
;; @001d                               v13 = uextend.i64 v2
;; @001d                               v15 = iadd v14, v13
;; @001d                               v16 = iconst.i64 4
;; @001d                               v17 = iadd v15, v16  ; v16 = 4
;; @001d                               v18 = load.i32 notrap aligned readonly v17
;; @001d                               v11 = load.i64 notrap aligned readonly can_move v0+48
;; @001d                               v12 = load.i32 notrap aligned readonly can_move v11
;; @001d                               v19 = icmp eq v18, v12
;; @001d                               v20 = uextend.i32 v19
;; @001d                               brif v20, block6(v20), block5
;;
;;                                 block5:
;; @001d                               v22 = call fn0(v0, v18, v12)
;; @001d                               jump block6(v22)
;;
;;                                 block6(v23: i32):
;; @001d                               jump block4(v23)
;;
;;                                 block4(v24: i32):
;; @0020                               jump block1(v24)
;;
;;                                 block1(v3: i32):
;; @0020                               return v3
;; }
