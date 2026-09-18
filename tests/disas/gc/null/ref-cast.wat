;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $s (struct))
  (func (param anyref) (result (ref $s))
    (ref.cast (ref $s) (local.get 0))
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
;; @001e                               v3 = iconst.i32 0
;; @001e                               v4 = icmp eq v2, v3  ; v3 = 0
;; @001e                               brif v4, block4(v3), block2  ; v3 = 0
;;
;;                                 block2:
;; @001e                               v7 = iconst.i32 1
;; @001e                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v34 = iconst.i32 0
;; @001e                               brif v8, block4(v34), block3  ; v34 = 0
;;
;;                                 block3:
;; @001e                               v11 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001e                               v12 = load.i64 notrap aligned readonly can_move region2 v11+32
;; @001e                               v10 = uextend.i64 v2
;; @001e                               v13 = iadd v12, v10
;; @001e                               v16 = load.i32 user2 readonly region4 v13
;; @001e                               v17 = iconst.i32 -1342177280
;; @001e                               v18 = band v16, v17  ; v17 = -1342177280
;; @001e                               v19 = icmp eq v18, v17  ; v17 = -1342177280
;;                                     v35 = iconst.i32 0
;; @001e                               brif v19, block5, block4(v35)  ; v35 = 0
;;
;;                                 block5:
;; @001e                               v28 = iconst.i64 4
;; @001e                               v29 = iadd.i64 v13, v28  ; v28 = 4
;; @001e                               v30 = load.i32 user2 readonly region7 v29
;; @001e                               v22 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @001e                               v23 = load.i32 notrap aligned readonly can_move region6 v22
;; @001e                               v31 = icmp eq v30, v23
;; @001e                               v32 = uextend.i32 v31
;; @001e                               jump block4(v32)
;;
;;                                 block4(v33: i32):
;; @001e                               trapz v33, user19
;; @0021                               jump block1
;;
;;                                 block1:
;; @0021                               return v2
;; }
