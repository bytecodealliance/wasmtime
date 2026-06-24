;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null -O gc-heap-reservation=0 -O gc-heap-guard-size=0"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (func (param (ref null $ty)) (result f32)
    (struct.get $ty 0 (local.get 0))
  )

  (func (param (ref null $ty)) (result i32)
    (struct.get_s $ty 1 (local.get 0))
  )

  (func (param (ref null $ty)) (result i32)
    (struct.get_u $ty 1 (local.get 0))
  )

  (func (param (ref null $ty)) (result anyref)
    (struct.get $ty 2 (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> f32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0033                               trapz v2, user16
;; @0033                               v3 = uextend.i64 v2
;; @0033                               v4 = iconst.i64 24
;; @0033                               v5 = uadd_overflow_trap v3, v4, user2  ; v4 = 24
;; @0033                               v6 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0033                               v7 = load.i64 notrap aligned region3 v6+40
;; @0033                               v10 = load.i64 notrap aligned region2 v6+32
;; @0033                               v8 = icmp ugt v5, v7
;; @0033                               v12 = iconst.i64 0
;; @0033                               v11 = iadd v10, v3
;; @0033                               v13 = select_spectre_guard v8, v12, v11  ; v12 = 0
;; @0033                               v14 = iconst.i64 8
;; @0033                               v15 = iadd v13, v14  ; v14 = 8
;; @0033                               v16 = load.f32 user2 little region4 v15
;; @0037                               jump block1
;;
;;                                 block1:
;; @0037                               return v16
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003c                               trapz v2, user16
;; @003c                               v3 = uextend.i64 v2
;; @003c                               v4 = iconst.i64 24
;; @003c                               v5 = uadd_overflow_trap v3, v4, user2  ; v4 = 24
;; @003c                               v6 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003c                               v7 = load.i64 notrap aligned region3 v6+40
;; @003c                               v10 = load.i64 notrap aligned region2 v6+32
;; @003c                               v8 = icmp ugt v5, v7
;; @003c                               v12 = iconst.i64 0
;; @003c                               v11 = iadd v10, v3
;; @003c                               v13 = select_spectre_guard v8, v12, v11  ; v12 = 0
;; @003c                               v14 = iconst.i64 12
;; @003c                               v15 = iadd v13, v14  ; v14 = 12
;; @003c                               v16 = load.i8 user2 little region4 v15
;; @0040                               jump block1
;;
;;                                 block1:
;; @003c                               v17 = sextend.i32 v16
;; @0040                               return v17
;; }
;;
;; function u0:2(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0045                               trapz v2, user16
;; @0045                               v3 = uextend.i64 v2
;; @0045                               v4 = iconst.i64 24
;; @0045                               v5 = uadd_overflow_trap v3, v4, user2  ; v4 = 24
;; @0045                               v6 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0045                               v7 = load.i64 notrap aligned region3 v6+40
;; @0045                               v10 = load.i64 notrap aligned region2 v6+32
;; @0045                               v8 = icmp ugt v5, v7
;; @0045                               v12 = iconst.i64 0
;; @0045                               v11 = iadd v10, v3
;; @0045                               v13 = select_spectre_guard v8, v12, v11  ; v12 = 0
;; @0045                               v14 = iconst.i64 12
;; @0045                               v15 = iadd v13, v14  ; v14 = 12
;; @0045                               v16 = load.i8 user2 little region4 v15
;; @0049                               jump block1
;;
;;                                 block1:
;; @0045                               v17 = uextend.i32 v16
;; @0049                               return v17
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @004e                               trapz v2, user16
;; @004e                               v3 = uextend.i64 v2
;; @004e                               v4 = iconst.i64 24
;; @004e                               v5 = uadd_overflow_trap v3, v4, user2  ; v4 = 24
;; @004e                               v6 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004e                               v7 = load.i64 notrap aligned region3 v6+40
;; @004e                               v10 = load.i64 notrap aligned region2 v6+32
;; @004e                               v8 = icmp ugt v5, v7
;; @004e                               v12 = iconst.i64 0
;; @004e                               v11 = iadd v10, v3
;; @004e                               v13 = select_spectre_guard v8, v12, v11  ; v12 = 0
;; @004e                               v14 = iconst.i64 16
;; @004e                               v15 = iadd v13, v14  ; v14 = 16
;; @004e                               v16 = load.i32 user2 little region4 v15
;; @0052                               jump block1
;;
;;                                 block1:
;; @0052                               return v16
;; }
