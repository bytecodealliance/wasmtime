;;! target = "x86_64"
;;! flags = "-W function-references,gc"
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
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0033                               brif v2, block3, block2
;;
;;                                 block2 cold:
;; @0033                               trap null_reference
;;
;;                                 block3:
;; @0033                               v7 = uextend.i64 v2
;; @0033                               v8 = iconst.i64 16
;; @0033                               v9 = uadd_overflow_trap v7, v8, user65535  ; v8 = 16
;; @0033                               v10 = iconst.i64 4
;; @0033                               v11 = uadd_overflow_trap v9, v10, user65535  ; v10 = 4
;; @0033                               v6 = load.i64 notrap aligned readonly v0+48
;; @0033                               v12 = icmp ult v11, v6
;; @0033                               brif v12, block5, block4
;;
;;                                 block4 cold:
;; @0033                               trap user65535
;;
;;                                 block5:
;; @0033                               v5 = load.i64 notrap aligned readonly v0+40
;; @0033                               v13 = iadd v5, v9
;; @0033                               v14 = load.f32 notrap aligned v13
;; @0037                               jump block1
;;
;;                                 block1:
;; @0037                               return v14
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003c                               brif v2, block3, block2
;;
;;                                 block2 cold:
;; @003c                               trap null_reference
;;
;;                                 block3:
;; @003c                               v7 = uextend.i64 v2
;; @003c                               v8 = iconst.i64 20
;; @003c                               v9 = uadd_overflow_trap v7, v8, user65535  ; v8 = 20
;; @003c                               v10 = iconst.i64 1
;; @003c                               v11 = uadd_overflow_trap v9, v10, user65535  ; v10 = 1
;; @003c                               v6 = load.i64 notrap aligned readonly v0+48
;; @003c                               v12 = icmp ult v11, v6
;; @003c                               brif v12, block5, block4
;;
;;                                 block4 cold:
;; @003c                               trap user65535
;;
;;                                 block5:
;; @003c                               v5 = load.i64 notrap aligned readonly v0+40
;; @003c                               v13 = iadd v5, v9
;; @003c                               v14 = load.i8 notrap aligned v13
;; @0040                               jump block1
;;
;;                                 block1:
;; @003c                               v15 = sextend.i32 v14
;; @0040                               return v15
;; }
;;
;; function u0:2(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0045                               brif v2, block3, block2
;;
;;                                 block2 cold:
;; @0045                               trap null_reference
;;
;;                                 block3:
;; @0045                               v7 = uextend.i64 v2
;; @0045                               v8 = iconst.i64 20
;; @0045                               v9 = uadd_overflow_trap v7, v8, user65535  ; v8 = 20
;; @0045                               v10 = iconst.i64 1
;; @0045                               v11 = uadd_overflow_trap v9, v10, user65535  ; v10 = 1
;; @0045                               v6 = load.i64 notrap aligned readonly v0+48
;; @0045                               v12 = icmp ult v11, v6
;; @0045                               brif v12, block5, block4
;;
;;                                 block4 cold:
;; @0045                               trap user65535
;;
;;                                 block5:
;; @0045                               v5 = load.i64 notrap aligned readonly v0+40
;; @0045                               v13 = iadd v5, v9
;; @0045                               v14 = load.i8 notrap aligned v13
;; @0049                               jump block1
;;
;;                                 block1:
;; @0045                               v15 = uextend.i32 v14
;; @0049                               return v15
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32) -> i32 system_v
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @004e                               brif v2, block7, block6
;;
;;                                 block6 cold:
;; @004e                               trap null_reference
;;
;;                                 block7:
;; @004e                               v7 = uextend.i64 v2
;; @004e                               v8 = iconst.i64 24
;; @004e                               v9 = uadd_overflow_trap v7, v8, user65535  ; v8 = 24
;; @004e                               v10 = iconst.i64 4
;; @004e                               v11 = uadd_overflow_trap v9, v10, user65535  ; v10 = 4
;; @004e                               v6 = load.i64 notrap aligned readonly v0+48
;; @004e                               v12 = icmp ult v11, v6
;; @004e                               brif v12, block9, block8
;;
;;                                 block8 cold:
;; @004e                               trap user65535
;;
;;                                 block9:
;; @004e                               v5 = load.i64 notrap aligned readonly v0+40
;; @004e                               v13 = iadd v5, v9
;; @004e                               v14 = load.i32 notrap aligned v13
;;                                     v54 = stack_addr.i64 ss0
;;                                     store notrap v14, v54
;; @004e                               v15 = iconst.i32 -2
;; @004e                               v16 = band v14, v15  ; v15 = -2
;;                                     v56 = iconst.i32 0
;; @004e                               v17 = icmp eq v16, v56  ; v56 = 0
;; @004e                               brif v17, block5, block2
;;
;;                                 block2:
;; @004e                               v19 = load.i64 notrap aligned v0+56
;; @004e                               v20 = load.i64 notrap aligned v19
;; @004e                               v21 = load.i64 notrap aligned v19+8
;; @004e                               v22 = icmp eq v20, v21
;; @004e                               brif v22, block3, block4
;;
;;                                 block4:
;; @004e                               v26 = uextend.i64 v14
;; @004e                               v27 = iconst.i64 8
;; @004e                               v28 = uadd_overflow_trap v26, v27, user65535  ; v27 = 8
;; @004e                               v30 = uadd_overflow_trap v28, v27, user65535  ; v27 = 8
;; @004e                               v31 = icmp ult v30, v6
;; @004e                               brif v31, block11, block10
;;
;;                                 block10 cold:
;; @004e                               trap user65535
;;
;;                                 block11:
;; @004e                               v32 = iadd.i64 v5, v28
;; @004e                               v33 = load.i64 notrap aligned v32
;;                                     v51 = load.i32 notrap v54
;; @004e                               v38 = uextend.i64 v51
;;                                     v64 = iconst.i64 8
;; @004e                               v40 = uadd_overflow_trap v38, v64, user65535  ; v64 = 8
;; @004e                               v42 = uadd_overflow_trap v40, v64, user65535  ; v64 = 8
;; @004e                               v43 = icmp ult v42, v6
;; @004e                               brif v43, block13, block12
;;
;;                                 block12 cold:
;; @004e                               trap user65535
;;
;;                                 block13:
;;                                     v58 = iconst.i64 1
;; @004e                               v34 = iadd.i64 v33, v58  ; v58 = 1
;; @004e                               v44 = iadd.i64 v5, v40
;; @004e                               store notrap aligned v34, v44
;;                                     v50 = load.i32 notrap v54
;; @004e                               store notrap aligned v50, v20
;;                                     v65 = iconst.i64 4
;;                                     v66 = iadd.i64 v20, v65  ; v65 = 4
;; @004e                               store notrap aligned v66, v19
;; @004e                               jump block5
;;
;;                                 block3 cold:
;; @004e                               v47 = call fn0(v0, v14), stack_map=[i32 @ ss0+0]
;; @004e                               jump block5
;;
;;                                 block5:
;;                                     v48 = load.i32 notrap v54
;; @0052                               jump block1
;;
;;                                 block1:
;; @0052                               return v48
;; }
