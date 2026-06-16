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
;; @0024                               v4 = iconst.i32 0
;; @0024                               v5 = icmp eq v2, v4  ; v4 = 0
;; @0024                               brif v5, block4(v4), block2  ; v4 = 0
;;
;;                                 block2:
;; @0024                               v8 = iconst.i32 1
;; @0024                               v9 = band.i32 v2, v8  ; v8 = 1
;;                                     v23 = iconst.i32 0
;; @0024                               brif v9, block4(v23), block3  ; v23 = 0
;;
;;                                 block3:
;; @0024                               v14 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0024                               v15 = load.i64 notrap aligned readonly can_move region3 v14+32
;; @0024                               v13 = uextend.i64 v2
;; @0024                               v16 = iadd v15, v13
;; @0024                               v17 = iconst.i64 4
;; @0024                               v18 = iadd v16, v17  ; v17 = 4
;; @0024                               v19 = load.i32 user2 readonly region5 v18
;; @0024                               v11 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0024                               v12 = load.i32 notrap aligned readonly can_move v11
;; @0024                               v20 = icmp eq v19, v12
;; @0024                               v21 = uextend.i32 v20
;; @0024                               jump block4(v21)
;;
;;                                 block4(v22: i32):
;; @0027                               jump block1(v22)
;;
;;                                 block1(v3: i32):
;; @0027                               return v3
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
;; @002c                               v4 = iconst.i32 0
;; @002c                               v5 = icmp eq v2, v4  ; v4 = 0
;; @002c                               brif v5, block4(v4), block2  ; v4 = 0
;;
;;                                 block2:
;; @002c                               v8 = iconst.i32 1
;; @002c                               v9 = band.i32 v2, v8  ; v8 = 1
;;                                     v23 = iconst.i32 0
;; @002c                               brif v9, block4(v23), block3  ; v23 = 0
;;
;;                                 block3:
;; @002c                               v14 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002c                               v15 = load.i64 notrap aligned readonly can_move region3 v14+32
;; @002c                               v13 = uextend.i64 v2
;; @002c                               v16 = iadd v15, v13
;; @002c                               v17 = iconst.i64 4
;; @002c                               v18 = iadd v16, v17  ; v17 = 4
;; @002c                               v19 = load.i32 user2 readonly region5 v18
;; @002c                               v11 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @002c                               v12 = load.i32 notrap aligned readonly can_move v11
;; @002c                               v20 = icmp eq v19, v12
;; @002c                               v21 = uextend.i32 v20
;; @002c                               jump block4(v21)
;;
;;                                 block4(v22: i32):
;; @002c                               trapz v22, user19
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return v2
;; }
