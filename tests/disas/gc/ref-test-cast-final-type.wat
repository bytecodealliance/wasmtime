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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 268435488 "VMStoreContext+0x20"
;;     region4 = 268435496 "VMStoreContext+0x28"
;;     region5 = 2147483648 "GcHeap"
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
;;                                     v22 = iconst.i32 0
;; @0024                               brif v8, block4(v22), block3  ; v22 = 0
;;
;;                                 block3:
;; @0024                               v13 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0024                               v14 = load.i64 notrap aligned readonly can_move region3 v13+32
;; @0024                               v12 = uextend.i64 v2
;; @0024                               v15 = iadd v14, v12
;; @0024                               v16 = iconst.i64 4
;; @0024                               v17 = iadd v15, v16  ; v16 = 4
;; @0024                               v18 = load.i32 user2 readonly region5 v17
;; @0024                               v10 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0024                               v11 = load.i32 notrap aligned readonly can_move v10
;; @0024                               v19 = icmp eq v18, v11
;; @0024                               v20 = uextend.i32 v19
;; @0024                               jump block4(v20)
;;
;;                                 block4(v21: i32):
;; @0027                               jump block1
;;
;;                                 block1:
;; @0027                               return v21
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 268435488 "VMStoreContext+0x20"
;;     region4 = 268435496 "VMStoreContext+0x28"
;;     region5 = 2147483648 "GcHeap"
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
;;                                     v22 = iconst.i32 0
;; @002c                               brif v8, block4(v22), block3  ; v22 = 0
;;
;;                                 block3:
;; @002c                               v13 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002c                               v14 = load.i64 notrap aligned readonly can_move region3 v13+32
;; @002c                               v12 = uextend.i64 v2
;; @002c                               v15 = iadd v14, v12
;; @002c                               v16 = iconst.i64 4
;; @002c                               v17 = iadd v15, v16  ; v16 = 4
;; @002c                               v18 = load.i32 user2 readonly region5 v17
;; @002c                               v10 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @002c                               v11 = load.i32 notrap aligned readonly can_move v10
;; @002c                               v19 = icmp eq v18, v11
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
