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
;; @0033                               trapz v2, user16
;; @0033                               v10 = load.i64 notrap aligned readonly can_move v0+8
;; @0033                               v5 = load.i64 notrap aligned readonly can_move v10+32
;; @0033                               v4 = uextend.i64 v2
;; @0033                               v6 = iadd v5, v4
;; @0033                               v7 = iconst.i64 24
;; @0033                               v8 = iadd v6, v7  ; v7 = 24
;; @0033                               v9 = load.f32 user2 little region0 v8
;; @0037                               jump block1
;;
;;                                 block1:
;; @0037                               return v9
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
;; @003c                               trapz v2, user16
;; @003c                               v11 = load.i64 notrap aligned readonly can_move v0+8
;; @003c                               v5 = load.i64 notrap aligned readonly can_move v11+32
;; @003c                               v4 = uextend.i64 v2
;; @003c                               v6 = iadd v5, v4
;; @003c                               v7 = iconst.i64 28
;; @003c                               v8 = iadd v6, v7  ; v7 = 28
;; @003c                               v9 = load.i8 user2 little region0 v8
;; @0040                               jump block1
;;
;;                                 block1:
;; @003c                               v10 = sextend.i32 v9
;; @0040                               return v10
;; }
;;
;; function u0:2(i64 vmctx, i64, i32) -> i32 tail {
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
;; @0045                               trapz v2, user16
;; @0045                               v11 = load.i64 notrap aligned readonly can_move v0+8
;; @0045                               v5 = load.i64 notrap aligned readonly can_move v11+32
;; @0045                               v4 = uextend.i64 v2
;; @0045                               v6 = iadd v5, v4
;; @0045                               v7 = iconst.i64 28
;; @0045                               v8 = iadd v6, v7  ; v7 = 28
;; @0045                               v9 = load.i8 user2 little region0 v8
;; @0049                               jump block1
;;
;;                                 block1:
;; @0045                               v10 = uextend.i32 v9
;; @0049                               return v10
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 2147483648 "GcHeap"
;;     region1 = 32 "VMContext+0x20"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:45 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @004e                               trapz v2, user16
;; @004e                               v95 = load.i64 notrap aligned readonly can_move v0+8
;; @004e                               v5 = load.i64 notrap aligned readonly can_move v95+32
;; @004e                               v4 = uextend.i64 v2
;; @004e                               v6 = iadd v5, v4
;; @004e                               v7 = iconst.i64 32
;; @004e                               v8 = iadd v6, v7  ; v7 = 32
;; @004e                               v9 = load.i32 user2 little region0 v8
;;                                     v84 = stack_addr.i64 ss0
;;                                     store notrap v9, v84
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
;; @004e                               v19 = load.i32 user2 region0 v18
;; @004e                               v20 = iconst.i32 2
;; @004e                               v21 = band v19, v20  ; v20 = 2
;; @004e                               brif v21, block4, block3
;;
;;                                 block3:
;; @004e                               v23 = load.i64 notrap aligned readonly can_move region1 v0+32
;; @004e                               v24 = load.i32 user2 region0 v23
;; @004e                               v28 = iconst.i64 16
;; @004e                               v29 = iadd.i64 v18, v28  ; v28 = 16
;; @004e                               store user2 region0 v24, v29
;;                                     v97 = iconst.i32 2
;;                                     v98 = bor.i32 v19, v97  ; v97 = 2
;; @004e                               store user2 region0 v98, v18
;; @004e                               v38 = iconst.i64 8
;; @004e                               v39 = iadd.i64 v18, v38  ; v38 = 8
;; @004e                               v40 = load.i64 user2 region0 v39
;; @004e                               v41 = iconst.i64 1
;; @004e                               v42 = iadd v40, v41  ; v41 = 1
;; @004e                               store user2 region0 v42, v39
;; @004e                               store.i32 user2 region0 v9, v23
;; @004e                               v50 = load.i32 notrap aligned v23+4
;;                                     v99 = iconst.i32 1
;;                                     v100 = iadd v50, v99  ; v99 = 1
;; @004e                               store notrap aligned v100, v23+4
;; @004e                               v60 = load.i32 notrap aligned v23+8
;; @004e                               v61 = iadd v60, v60
;; @004e                               v62 = iconst.i32 1024
;; @004e                               v63 = umax v61, v62  ; v62 = 1024
;; @004e                               v64 = icmp uge v100, v63
;; @004e                               brif v64, block5, block6
;;
;;                                 block5 cold:
;; @004e                               v65 = call fn0(v0), stack_map=[i32 @ ss0+0]
;; @004e                               jump block6
;;
;;                                 block6:
;; @004e                               jump block4
;;
;;                                 block4:
;;                                     v67 = load.i32 notrap v84
;; @0052                               jump block1
;;
;;                                 block1:
;; @0052                               return v67
;; }
