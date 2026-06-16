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
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0033                               trapz v2, user16
;; @0033                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0033                               v6 = load.i64 notrap aligned readonly can_move v5+32
;; @0033                               v4 = uextend.i64 v2
;; @0033                               v7 = iadd v6, v4
;; @0033                               v8 = iconst.i64 24
;; @0033                               v9 = iadd v7, v8  ; v8 = 24
;; @0033                               v10 = load.f32 user2 little region1 v9
;; @0037                               jump block1
;;
;;                                 block1:
;; @0037                               return v10
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003c                               trapz v2, user16
;; @003c                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003c                               v6 = load.i64 notrap aligned readonly can_move v5+32
;; @003c                               v4 = uextend.i64 v2
;; @003c                               v7 = iadd v6, v4
;; @003c                               v8 = iconst.i64 28
;; @003c                               v9 = iadd v7, v8  ; v8 = 28
;; @003c                               v10 = load.i8 user2 little region1 v9
;; @0040                               jump block1
;;
;;                                 block1:
;; @003c                               v11 = sextend.i32 v10
;; @0040                               return v11
;; }
;;
;; function u0:2(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0045                               trapz v2, user16
;; @0045                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0045                               v6 = load.i64 notrap aligned readonly can_move v5+32
;; @0045                               v4 = uextend.i64 v2
;; @0045                               v7 = iadd v6, v4
;; @0045                               v8 = iconst.i64 28
;; @0045                               v9 = iadd v7, v8  ; v8 = 28
;; @0045                               v10 = load.i8 user2 little region1 v9
;; @0049                               jump block1
;;
;;                                 block1:
;; @0045                               v11 = uextend.i32 v10
;; @0049                               return v11
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     region2 = 32 "VMContext+0x20"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:45 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @004e                               trapz v2, user16
;; @004e                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004e                               v6 = load.i64 notrap aligned readonly can_move v5+32
;; @004e                               v4 = uextend.i64 v2
;; @004e                               v7 = iadd v6, v4
;; @004e                               v8 = iconst.i64 32
;; @004e                               v9 = iadd v7, v8  ; v8 = 32
;; @004e                               v10 = load.i32 user2 little region1 v9
;;                                     v85 = stack_addr.i64 ss0
;;                                     store notrap v10, v85
;; @004e                               v11 = iconst.i32 1
;; @004e                               v12 = band v10, v11  ; v11 = 1
;; @004e                               v13 = iconst.i32 0
;; @004e                               v14 = icmp eq v10, v13  ; v13 = 0
;; @004e                               v15 = uextend.i32 v14
;; @004e                               v16 = bor v12, v15
;; @004e                               brif v16, block4, block2
;;
;;                                 block2:
;; @004e                               v17 = uextend.i64 v10
;; @004e                               v20 = iadd.i64 v6, v17
;; @004e                               v21 = load.i32 user2 region1 v20
;; @004e                               v22 = iconst.i32 2
;; @004e                               v23 = band v21, v22  ; v22 = 2
;; @004e                               brif v23, block4, block3
;;
;;                                 block3:
;; @004e                               v24 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @004e                               v25 = load.i32 user2 region1 v24
;; @004e                               v30 = iconst.i64 16
;; @004e                               v31 = iadd.i64 v20, v30  ; v30 = 16
;; @004e                               store user2 region1 v25, v31
;;                                     v86 = iconst.i32 2
;;                                     v87 = bor.i32 v21, v86  ; v86 = 2
;; @004e                               store user2 region1 v87, v20
;; @004e                               v42 = iconst.i64 8
;; @004e                               v43 = iadd.i64 v20, v42  ; v42 = 8
;; @004e                               v44 = load.i64 user2 region1 v43
;; @004e                               v45 = iconst.i64 1
;; @004e                               v46 = iadd v44, v45  ; v45 = 1
;; @004e                               store user2 region1 v46, v43
;; @004e                               store.i32 user2 region1 v10, v24
;; @004e                               v54 = load.i32 notrap aligned v24+4
;;                                     v88 = iconst.i32 1
;;                                     v89 = iadd v54, v88  ; v88 = 1
;; @004e                               store notrap aligned v89, v24+4
;; @004e                               v61 = load.i32 notrap aligned v24+8
;; @004e                               v62 = iadd v61, v61
;; @004e                               v63 = iconst.i32 1024
;; @004e                               v64 = umax v62, v63  ; v63 = 1024
;; @004e                               v65 = icmp uge v89, v64
;; @004e                               brif v65, block5, block6
;;
;;                                 block5 cold:
;; @004e                               v66 = call fn0(v0), stack_map=[i32 @ ss0+0]
;; @004e                               jump block6
;;
;;                                 block6:
;; @004e                               jump block4
;;
;;                                 block4:
;;                                     v68 = load.i32 notrap v85
;; @0052                               jump block1
;;
;;                                 block1:
;; @0052                               return v68
;; }
