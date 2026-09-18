;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $s (struct))
  (func (param anyref) (result i32)
    (ref.test (ref $s) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 196 ""
;;     region3 = 206 ""
;;     region4 = 239 ""
;;     region5 = 130 ""
;;     region6 = 6 ""
;;     region7 = 134 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001d                               v3 = iconst.i32 0
;; @001d                               v4 = icmp eq v2, v3  ; v3 = 0
;; @001d                               brif v4, block4(v3), block2  ; v3 = 0
;;
;;                                 block2:
;; @001d                               v7 = iconst.i32 1
;; @001d                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v34 = iconst.i32 0
;; @001d                               brif v8, block4(v34), block3  ; v34 = 0
;;
;;                                 block3:
;; @001d                               v11 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001d                               v12 = load.i64 notrap aligned readonly can_move region2 v11+32
;; @001d                               v10 = uextend.i64 v2
;; @001d                               v13 = iadd v12, v10
;; @001d                               v16 = load.i32 user2 readonly region4 v13
;; @001d                               v17 = iconst.i32 -1342177280
;; @001d                               v18 = band v16, v17  ; v17 = -1342177280
;; @001d                               v19 = icmp eq v18, v17  ; v17 = -1342177280
;;                                     v35 = iconst.i32 0
;; @001d                               brif v19, block5, block4(v35)  ; v35 = 0
;;
;;                                 block5:
;; @001d                               v28 = iconst.i64 4
;; @001d                               v29 = iadd.i64 v13, v28  ; v28 = 4
;; @001d                               v30 = load.i32 user2 readonly region7 v29
;; @001d                               v22 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @001d                               v23 = load.i32 notrap aligned readonly can_move region6 v22
;; @001d                               v31 = icmp eq v30, v23
;; @001d                               v32 = uextend.i32 v31
;; @001d                               jump block4(v32)
;;
;;                                 block4(v33: i32):
;; @0020                               jump block1
;;
;;                                 block1:
;; @0020                               return v33
;; }
