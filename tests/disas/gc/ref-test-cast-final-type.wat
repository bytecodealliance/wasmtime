;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

;; `ref.test` / `ref.cast` against a `final` concrete type, which allows us to
;; omit the slow-path from the subtype check.

(module
  (type $s (struct))   ;; final by default

  (func (param anyref) (result i32)
    (ref.test (ref $s) (local.get 0)))

  (func (param anyref) (result (ref $s))
    (ref.cast (ref $s) (local.get 0)))
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
;; @0024                               v3 = iconst.i32 0
;; @0024                               v4 = icmp eq v2, v3  ; v3 = 0
;; @0024                               brif v4, block4(v3), block2  ; v3 = 0
;;
;;                                 block2:
;; @0024                               v7 = iconst.i32 1
;; @0024                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v34 = iconst.i32 0
;; @0024                               brif v8, block4(v34), block3  ; v34 = 0
;;
;;                                 block3:
;; @0024                               v11 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0024                               v12 = load.i64 notrap aligned readonly can_move region2 v11+32
;; @0024                               v10 = uextend.i64 v2
;; @0024                               v13 = iadd v12, v10
;; @0024                               v16 = load.i32 user2 readonly region4 v13
;; @0024                               v17 = iconst.i32 -1342177280
;; @0024                               v18 = band v16, v17  ; v17 = -1342177280
;; @0024                               v19 = icmp eq v18, v17  ; v17 = -1342177280
;;                                     v35 = iconst.i32 0
;; @0024                               brif v19, block5, block4(v35)  ; v35 = 0
;;
;;                                 block5:
;; @0024                               v28 = iconst.i64 4
;; @0024                               v29 = iadd.i64 v13, v28  ; v28 = 4
;; @0024                               v30 = load.i32 user2 readonly region7 v29
;; @0024                               v22 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @0024                               v23 = load.i32 notrap aligned readonly can_move region6 v22
;; @0024                               v31 = icmp eq v30, v23
;; @0024                               v32 = uextend.i32 v31
;; @0024                               jump block4(v32)
;;
;;                                 block4(v33: i32):
;; @0027                               jump block1
;;
;;                                 block1:
;; @0027                               return v33
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
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
;; @002c                               v3 = iconst.i32 0
;; @002c                               v4 = icmp eq v2, v3  ; v3 = 0
;; @002c                               brif v4, block4(v3), block2  ; v3 = 0
;;
;;                                 block2:
;; @002c                               v7 = iconst.i32 1
;; @002c                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v34 = iconst.i32 0
;; @002c                               brif v8, block4(v34), block3  ; v34 = 0
;;
;;                                 block3:
;; @002c                               v11 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002c                               v12 = load.i64 notrap aligned readonly can_move region2 v11+32
;; @002c                               v10 = uextend.i64 v2
;; @002c                               v13 = iadd v12, v10
;; @002c                               v16 = load.i32 user2 readonly region4 v13
;; @002c                               v17 = iconst.i32 -1342177280
;; @002c                               v18 = band v16, v17  ; v17 = -1342177280
;; @002c                               v19 = icmp eq v18, v17  ; v17 = -1342177280
;;                                     v35 = iconst.i32 0
;; @002c                               brif v19, block5, block4(v35)  ; v35 = 0
;;
;;                                 block5:
;; @002c                               v28 = iconst.i64 4
;; @002c                               v29 = iadd.i64 v13, v28  ; v28 = 4
;; @002c                               v30 = load.i32 user2 readonly region7 v29
;; @002c                               v22 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @002c                               v23 = load.i32 notrap aligned readonly can_move region6 v22
;; @002c                               v31 = icmp eq v30, v23
;; @002c                               v32 = uextend.i32 v31
;; @002c                               jump block4(v32)
;;
;;                                 block4(v33: i32):
;; @002c                               trapz v33, user19
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return v2
;; }
