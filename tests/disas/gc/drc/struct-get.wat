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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0033                               trapz v2, user16
;; @0033                               v10 = load.i64 notrap aligned readonly can_move v0+8
;; @0033                               v5 = load.i64 notrap aligned readonly can_move v10+24
;; @0033                               v4 = uextend.i64 v2
;; @0033                               v6 = iadd v5, v4
;; @0033                               v7 = iconst.i64 16
;; @0033                               v8 = iadd v6, v7  ; v7 = 16
;; @0033                               v9 = load.f32 notrap aligned little v8
;; @0037                               jump block1
;;
;;                                 block1:
;; @0037                               return v9
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003c                               trapz v2, user16
;; @003c                               v11 = load.i64 notrap aligned readonly can_move v0+8
;; @003c                               v5 = load.i64 notrap aligned readonly can_move v11+24
;; @003c                               v4 = uextend.i64 v2
;; @003c                               v6 = iadd v5, v4
;; @003c                               v7 = iconst.i64 20
;; @003c                               v8 = iadd v6, v7  ; v7 = 20
;; @003c                               v9 = load.i8 notrap aligned little v8
;; @0040                               jump block1
;;
;;                                 block1:
;; @003c                               v10 = sextend.i32 v9
;; @0040                               return v10
;; }
;;
;; function u0:2(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0045                               trapz v2, user16
;; @0045                               v11 = load.i64 notrap aligned readonly can_move v0+8
;; @0045                               v5 = load.i64 notrap aligned readonly can_move v11+24
;; @0045                               v4 = uextend.i64 v2
;; @0045                               v6 = iadd v5, v4
;; @0045                               v7 = iconst.i64 20
;; @0045                               v8 = iadd v6, v7  ; v7 = 20
;; @0045                               v9 = load.i8 notrap aligned little v8
;; @0049                               jump block1
;;
;;                                 block1:
;; @0045                               v10 = uextend.i32 v9
;; @0049                               return v10
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @004e                               trapz v2, user16
;; @004e                               v57 = load.i64 notrap aligned readonly can_move v0+8
;; @004e                               v5 = load.i64 notrap aligned readonly can_move v57+24
;; @004e                               v4 = uextend.i64 v2
;; @004e                               v6 = iadd v5, v4
;; @004e                               v7 = iconst.i64 24
;; @004e                               v8 = iadd v6, v7  ; v7 = 24
;; @004e                               v9 = load.i32 notrap aligned little v8
;;                                     v56 = stack_addr.i64 ss0
;;                                     store notrap v9, v56
;;                                     v54 = iconst.i32 1
;; @004e                               v10 = band v9, v54  ; v54 = 1
;;                                     v52 = iconst.i32 0
;; @004e                               v11 = icmp eq v9, v52  ; v52 = 0
;; @004e                               v12 = uextend.i32 v11
;; @004e                               v13 = bor v10, v12
;; @004e                               brif v13, block5, block2
;;
;;                                 block2:
;; @004e                               v15 = load.i64 notrap aligned readonly v0+32
;; @004e                               v16 = load.i64 notrap aligned v15
;; @004e                               v17 = load.i64 notrap aligned v15+8
;; @004e                               v18 = icmp eq v16, v17
;; @004e                               brif v18, block3, block4
;;
;;                                 block4:
;; @004e                               v19 = uextend.i64 v9
;; @004e                               v21 = iadd.i64 v5, v19
;; @004e                               v22 = iconst.i64 8
;; @004e                               v23 = iadd v21, v22  ; v22 = 8
;; @004e                               v24 = load.i64 notrap aligned v23
;;                                     v48 = iconst.i64 1
;; @004e                               v25 = iadd v24, v48  ; v48 = 1
;; @004e                               store notrap aligned v25, v23
;;                                     v36 = load.i32 notrap v56
;; @004e                               store notrap aligned v36, v16
;;                                     v43 = iconst.i64 4
;; @004e                               v31 = iadd.i64 v16, v43  ; v43 = 4
;; @004e                               store notrap aligned v31, v15
;; @004e                               jump block5
;;
;;                                 block3 cold:
;; @004e                               v33 = call fn0(v0, v9), stack_map=[i32 @ ss0+0]
;; @004e                               jump block5
;;
;;                                 block5:
;;                                     v34 = load.i32 notrap v56
;; @0052                               jump block1
;;
;;                                 block1:
;; @0052                               return v34
;; }
