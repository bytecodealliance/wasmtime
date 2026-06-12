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
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0033                               trapz v2, user16
;; @0033                               v10 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0033                               v5 = load.i64 notrap aligned readonly can_move v10+32
;; @0033                               v4 = uextend.i64 v2
;; @0033                               v6 = iadd v5, v4
;; @0033                               v7 = iconst.i64 24
;; @0033                               v8 = iadd v6, v7  ; v7 = 24
;; @0033                               v9 = load.f32 user2 little region1 v8
;; @0037                               jump block1
;;
;;                                 block1:
;; @0037                               return v9
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003c                               trapz v2, user16
;; @003c                               v11 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003c                               v5 = load.i64 notrap aligned readonly can_move v11+32
;; @003c                               v4 = uextend.i64 v2
;; @003c                               v6 = iadd v5, v4
;; @003c                               v7 = iconst.i64 28
;; @003c                               v8 = iadd v6, v7  ; v7 = 28
;; @003c                               v9 = load.i8 user2 little region1 v8
;; @0040                               jump block1
;;
;;                                 block1:
;; @003c                               v10 = sextend.i32 v9
;; @0040                               return v10
;; }
;;
;; function u0:2(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0045                               trapz v2, user16
;; @0045                               v11 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0045                               v5 = load.i64 notrap aligned readonly can_move v11+32
;; @0045                               v4 = uextend.i64 v2
;; @0045                               v6 = iadd v5, v4
;; @0045                               v7 = iconst.i64 28
;; @0045                               v8 = iadd v6, v7  ; v7 = 28
;; @0045                               v9 = load.i8 user2 little region1 v8
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
;;     region1 = 2147483648 "GcHeap"
;;     region2 = 32 "VMContext+0x20"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:45 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @004e                               trapz v2, user16
;; @004e                               v90 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004e                               v5 = load.i64 notrap aligned readonly can_move v90+32
;; @004e                               v4 = uextend.i64 v2
;; @004e                               v6 = iadd v5, v4
;; @004e                               v7 = iconst.i64 32
;; @004e                               v8 = iadd v6, v7  ; v7 = 32
;; @004e                               v9 = load.i32 user2 little region1 v8
;;                                     v79 = stack_addr.i64 ss0
;;                                     store notrap v9, v79
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
;; @004e                               v18 = iadd.i64 v5, v16
;; @004e                               v19 = load.i32 user2 region1 v18
;; @004e                               v20 = iconst.i32 2
;; @004e                               v21 = band v19, v20  ; v20 = 2
;; @004e                               brif v21, block4, block3
;;
;;                                 block3:
;; @004e                               v22 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @004e                               v23 = load.i32 user2 region1 v22
;; @004e                               v27 = iconst.i64 16
;; @004e                               v28 = iadd.i64 v18, v27  ; v27 = 16
;; @004e                               store user2 region1 v23, v28
;;                                     v92 = iconst.i32 2
;;                                     v93 = bor.i32 v19, v92  ; v92 = 2
;; @004e                               store user2 region1 v93, v18
;; @004e                               v37 = iconst.i64 8
;; @004e                               v38 = iadd.i64 v18, v37  ; v37 = 8
;; @004e                               v39 = load.i64 user2 region1 v38
;; @004e                               v40 = iconst.i64 1
;; @004e                               v41 = iadd v39, v40  ; v40 = 1
;; @004e                               store user2 region1 v41, v38
;; @004e                               store.i32 user2 region1 v9, v22
;; @004e                               v48 = load.i32 notrap aligned v22+4
;;                                     v94 = iconst.i32 1
;;                                     v95 = iadd v48, v94  ; v94 = 1
;; @004e                               store notrap aligned v95, v22+4
;; @004e                               v55 = load.i32 notrap aligned v22+8
;; @004e                               v56 = iadd v55, v55
;; @004e                               v57 = iconst.i32 1024
;; @004e                               v58 = umax v56, v57  ; v57 = 1024
;; @004e                               v59 = icmp uge v95, v58
;; @004e                               brif v59, block5, block6
;;
;;                                 block5 cold:
;; @004e                               v60 = call fn0(v0), stack_map=[i32 @ ss0+0]
;; @004e                               jump block6
;;
;;                                 block6:
;; @004e                               jump block4
;;
;;                                 block4:
;;                                     v62 = load.i32 notrap v79
;; @0052                               jump block1
;;
;;                                 block1:
;; @0052                               return v62
;; }
