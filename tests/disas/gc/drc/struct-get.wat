;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;; @0033                               v4 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0033                               v5 = load.i64 notrap aligned readonly can_move region2 v4+32
;; @0033                               v3 = uextend.i64 v2
;; @0033                               v6 = iadd v5, v3
;; @0033                               v7 = iconst.i64 24
;; @0033                               v8 = iadd v6, v7  ; v7 = 24
;; @0033                               v9 = load.f32 user2 little region4 v8
;; @0037                               jump block1
;;
;;                                 block1:
;; @0037                               return v9
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
;; @003c                               v4 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003c                               v5 = load.i64 notrap aligned readonly can_move region2 v4+32
;; @003c                               v3 = uextend.i64 v2
;; @003c                               v6 = iadd v5, v3
;; @003c                               v7 = iconst.i64 28
;; @003c                               v8 = iadd v6, v7  ; v7 = 28
;; @003c                               v9 = load.i8 user2 little region4 v8
;; @0040                               jump block1
;;
;;                                 block1:
;; @003c                               v10 = sextend.i32 v9
;; @0040                               return v10
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
;; @0045                               v4 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0045                               v5 = load.i64 notrap aligned readonly can_move region2 v4+32
;; @0045                               v3 = uextend.i64 v2
;; @0045                               v6 = iadd v5, v3
;; @0045                               v7 = iconst.i64 28
;; @0045                               v8 = iadd v6, v7  ; v7 = 28
;; @0045                               v9 = load.i8 user2 little region4 v8
;; @0049                               jump block1
;;
;;                                 block1:
;; @0045                               v10 = uextend.i32 v9
;; @0049                               return v10
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     region5 = 32 "VMContext+0x20"
;;     region6 = 3221225472 "VMDrcHeapData+0x0"
;;     region7 = 3221225476 "VMDrcHeapData+0x4"
;;     region8 = 3221225480 "VMDrcHeapData+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:45 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @004e                               trapz v2, user16
;; @004e                               v4 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004e                               v5 = load.i64 notrap aligned readonly can_move region2 v4+32
;; @004e                               v3 = uextend.i64 v2
;; @004e                               v6 = iadd v5, v3
;; @004e                               v7 = iconst.i64 32
;; @004e                               v8 = iadd v6, v7  ; v7 = 32
;; @004e                               v9 = load.i32 user2 little region4 v8
;;                                     v81 = stack_addr.i64 ss0
;;                                     store notrap v9, v81
;; @004e                               v10 = iconst.i32 1
;; @004e                               v11 = band v9, v10  ; v10 = 1
;; @004e                               v12 = iconst.i32 0
;; @004e                               v13 = icmp eq v9, v12  ; v12 = 0
;; @004e                               v14 = uextend.i32 v13
;; @004e                               v15 = bor v11, v14
;; @004e                               brif v15, block4, block2
;;
;;                                 block2:
;; @004e                               v16 = uextend.i64 v9
;; @004e                               v19 = iadd.i64 v5, v16
;; @004e                               v20 = load.i32 user2 region4 v19
;; @004e                               v21 = iconst.i32 2
;; @004e                               v22 = band v20, v21  ; v21 = 2
;; @004e                               brif v22, block4, block3
;;
;;                                 block3:
;; @004e                               v23 = load.i64 notrap aligned readonly can_move region5 v0+32
;; @004e                               v24 = load.i32 notrap aligned region6 v23
;; @004e                               v29 = iconst.i64 16
;; @004e                               v30 = iadd.i64 v19, v29  ; v29 = 16
;; @004e                               store user2 region4 v24, v30
;;                                     v82 = iconst.i32 2
;;                                     v83 = bor.i32 v20, v82  ; v82 = 2
;; @004e                               store user2 region4 v83, v19
;; @004e                               v41 = iconst.i64 8
;; @004e                               v42 = iadd.i64 v19, v41  ; v41 = 8
;; @004e                               v43 = load.i64 user2 region4 v42
;; @004e                               v44 = iconst.i64 1
;; @004e                               v45 = iadd v43, v44  ; v44 = 1
;; @004e                               store user2 region4 v45, v42
;; @004e                               store.i32 notrap aligned region6 v9, v23
;; @004e                               v52 = load.i32 notrap aligned region7 v23+4
;;                                     v84 = iconst.i32 1
;;                                     v85 = iadd v52, v84  ; v84 = 1
;; @004e                               store notrap aligned region7 v85, v23+4
;; @004e                               v57 = load.i32 notrap aligned region8 v23+8
;; @004e                               v58 = iadd v57, v57
;; @004e                               v59 = iconst.i32 1024
;; @004e                               v60 = umax v58, v59  ; v59 = 1024
;; @004e                               v61 = icmp uge v85, v60
;; @004e                               brif v61, block5, block6
;;
;;                                 block5 cold:
;; @004e                               v62 = call fn0(v0), stack_map=[i32 @ ss0+0]
;; @004e                               jump block6
;;
;;                                 block6:
;; @004e                               jump block4
;;
;;                                 block4:
;; @0052                               jump block1
;;
;;                                 block1:
;;                                     v64 = load.i32 notrap v81
;; @0052                               return v64
;; }
