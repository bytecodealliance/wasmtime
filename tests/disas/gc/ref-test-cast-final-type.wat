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
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0024                               v4 = iconst.i32 0
;; @0024                               v5 = icmp eq v2, v4  ; v4 = 0
;; @0024                               brif v5, block4(v4), block2  ; v4 = 0
;;
;;                                 block2:
;; @0024                               v8 = iconst.i32 1
;; @0024                               v9 = band.i32 v2, v8  ; v8 = 1
;;                                     v24 = iconst.i32 0
;; @0024                               brif v9, block4(v24), block3  ; v24 = 0
;;
;;                                 block3:
;; @0024                               v22 = load.i64 notrap aligned readonly can_move v0+8
;; @0024                               v14 = load.i64 notrap aligned readonly can_move v22+32
;; @0024                               v13 = uextend.i64 v2
;; @0024                               v15 = iadd v14, v13
;; @0024                               v16 = iconst.i64 4
;; @0024                               v17 = iadd v15, v16  ; v16 = 4
;; @0024                               v18 = load.i32 user2 readonly region0 v17
;; @0024                               v11 = load.i64 notrap aligned readonly can_move v0+40
;; @0024                               v12 = load.i32 notrap aligned readonly can_move v11
;; @0024                               v19 = icmp eq v18, v12
;; @0024                               v20 = uextend.i32 v19
;; @0024                               jump block4(v20)
;;
;;                                 block4(v21: i32):
;; @0027                               jump block1(v21)
;;
;;                                 block1(v3: i32):
;; @0027                               return v3
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002c                               v4 = iconst.i32 0
;; @002c                               v5 = icmp eq v2, v4  ; v4 = 0
;; @002c                               brif v5, block4(v4), block2  ; v4 = 0
;;
;;                                 block2:
;; @002c                               v8 = iconst.i32 1
;; @002c                               v9 = band.i32 v2, v8  ; v8 = 1
;;                                     v24 = iconst.i32 0
;; @002c                               brif v9, block4(v24), block3  ; v24 = 0
;;
;;                                 block3:
;; @002c                               v22 = load.i64 notrap aligned readonly can_move v0+8
;; @002c                               v14 = load.i64 notrap aligned readonly can_move v22+32
;; @002c                               v13 = uextend.i64 v2
;; @002c                               v15 = iadd v14, v13
;; @002c                               v16 = iconst.i64 4
;; @002c                               v17 = iadd v15, v16  ; v16 = 4
;; @002c                               v18 = load.i32 user2 readonly region0 v17
;; @002c                               v11 = load.i64 notrap aligned readonly can_move v0+40
;; @002c                               v12 = load.i32 notrap aligned readonly can_move v11
;; @002c                               v19 = icmp eq v18, v12
;; @002c                               v20 = uextend.i32 v19
;; @002c                               jump block4(v20)
;;
;;                                 block4(v21: i32):
;; @002c                               trapz v21, user19
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return v2
;; }
