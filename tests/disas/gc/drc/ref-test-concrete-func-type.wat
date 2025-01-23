;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $f (func (param i32) (result i32)))
  (func (param funcref) (result i32)
    (ref.test (ref $f) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32, i32) -> i32 tail
;;     fn0 = colocated u1:35 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;;                                     v19 = iconst.i64 0
;; @0020                               v4 = icmp eq v2, v19  ; v19 = 0
;; @0020                               v5 = uextend.i32 v4
;; @0020                               v7 = iconst.i32 1
;; @0020                               v6 = iconst.i32 0
;;                                     v24 = select v2, v7, v6  ; v7 = 1, v6 = 0
;; @0020                               brif v5, block4(v24), block2
;;
;;                                 block2:
;; @0020                               jump block3
;;
;;                                 block3:
;; @0020                               v12 = load.i32 notrap aligned readonly v2+16
;; @0020                               v10 = load.i64 notrap aligned readonly v0+64
;; @0020                               v11 = load.i32 notrap aligned readonly v10
;; @0020                               v13 = icmp eq v12, v11
;; @0020                               v14 = uextend.i32 v13
;; @0020                               brif v14, block6(v14), block5
;;
;;                                 block5:
;; @0020                               v16 = call fn0(v0, v12, v11)
;; @0020                               jump block6(v16)
;;
;;                                 block6(v17: i32):
;; @0020                               jump block4(v17)
;;
;;                                 block4(v18: i32):
;; @0023                               jump block1(v18)
;;
;;                                 block1(v3: i32):
;; @0023                               return v3
;; }
