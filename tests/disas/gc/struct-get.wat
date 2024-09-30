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
;; @0033                               trapz v2, null_reference
;; @0033                               v8 = uextend.i64 v2
;; @0033                               v9 = iconst.i64 16
;; @0033                               v10 = uadd_overflow_trap v8, v9, user65535  ; v9 = 16
;;                                     v16 = iconst.i64 32
;; @0033                               v12 = uadd_overflow_trap v8, v16, user65535  ; v16 = 32
;; @0033                               v7 = load.i64 notrap aligned readonly v0+48
;; @0033                               v13 = icmp ule v12, v7
;; @0033                               trapz v13, user65535
;; @0033                               v6 = load.i64 notrap aligned readonly v0+40
;; @0033                               v14 = iadd v6, v10
;; @0033                               v15 = load.f32 notrap aligned little v14
;; @0037                               jump block1
;;
;;                                 block1:
;; @0037                               return v15
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
;; @003c                               trapz v2, null_reference
;; @003c                               v8 = uextend.i64 v2
;; @003c                               v9 = iconst.i64 20
;; @003c                               v10 = uadd_overflow_trap v8, v9, user65535  ; v9 = 20
;;                                     v17 = iconst.i64 32
;; @003c                               v12 = uadd_overflow_trap v8, v17, user65535  ; v17 = 32
;; @003c                               v7 = load.i64 notrap aligned readonly v0+48
;; @003c                               v13 = icmp ule v12, v7
;; @003c                               trapz v13, user65535
;; @003c                               v6 = load.i64 notrap aligned readonly v0+40
;; @003c                               v14 = iadd v6, v10
;; @003c                               v15 = load.i8 notrap aligned little v14
;; @0040                               jump block1
;;
;;                                 block1:
;; @003c                               v16 = sextend.i32 v15
;; @0040                               return v16
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
;; @0045                               trapz v2, null_reference
;; @0045                               v8 = uextend.i64 v2
;; @0045                               v9 = iconst.i64 20
;; @0045                               v10 = uadd_overflow_trap v8, v9, user65535  ; v9 = 20
;;                                     v17 = iconst.i64 32
;; @0045                               v12 = uadd_overflow_trap v8, v17, user65535  ; v17 = 32
;; @0045                               v7 = load.i64 notrap aligned readonly v0+48
;; @0045                               v13 = icmp ule v12, v7
;; @0045                               trapz v13, user65535
;; @0045                               v6 = load.i64 notrap aligned readonly v0+40
;; @0045                               v14 = iadd v6, v10
;; @0045                               v15 = load.i8 notrap aligned little v14
;; @0049                               jump block1
;;
;;                                 block1:
;; @0045                               v16 = uextend.i32 v15
;; @0049                               return v16
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
;; @004e                               trapz v2, null_reference
;; @004e                               v8 = uextend.i64 v2
;; @004e                               v9 = iconst.i64 24
;; @004e                               v10 = uadd_overflow_trap v8, v9, user65535  ; v9 = 24
;;                                     v65 = iconst.i64 32
;; @004e                               v12 = uadd_overflow_trap v8, v65, user65535  ; v65 = 32
;; @004e                               v7 = load.i64 notrap aligned readonly v0+48
;; @004e                               v13 = icmp ule v12, v7
;; @004e                               trapz v13, user65535
;; @004e                               v6 = load.i64 notrap aligned readonly v0+40
;; @004e                               v14 = iadd v6, v10
;; @004e                               v15 = load.i32 notrap aligned little v14
;;                                     v55 = stack_addr.i64 ss0
;;                                     store notrap v15, v55
;; @004e                               v16 = iconst.i32 -2
;; @004e                               v17 = band v15, v16  ; v16 = -2
;;                                     v57 = iconst.i32 0
;; @004e                               v18 = icmp eq v17, v57  ; v57 = 0
;; @004e                               brif v18, block5, block2
;;
;;                                 block2:
;; @004e                               v20 = load.i64 notrap aligned v0+56
;; @004e                               v21 = load.i64 notrap aligned v20
;; @004e                               v22 = load.i64 notrap aligned v20+8
;; @004e                               v23 = icmp eq v21, v22
;; @004e                               brif v23, block3, block4
;;
;;                                 block4:
;; @004e                               v27 = uextend.i64 v15
;; @004e                               v28 = iconst.i64 8
;; @004e                               v29 = uadd_overflow_trap v27, v28, user65535  ; v28 = 8
;; @004e                               v31 = uadd_overflow_trap v29, v28, user65535  ; v28 = 8
;; @004e                               v32 = icmp ule v31, v7
;; @004e                               trapz v32, user65535
;; @004e                               v33 = iadd.i64 v6, v29
;; @004e                               v34 = load.i64 notrap aligned v33
;;                                     v52 = load.i32 notrap v55
;; @004e                               v39 = uextend.i64 v52
;; @004e                               v41 = uadd_overflow_trap v39, v28, user65535  ; v28 = 8
;; @004e                               v43 = uadd_overflow_trap v41, v28, user65535  ; v28 = 8
;; @004e                               v44 = icmp ule v43, v7
;; @004e                               trapz v44, user65535
;;                                     v59 = iconst.i64 1
;; @004e                               v35 = iadd v34, v59  ; v59 = 1
;; @004e                               v45 = iadd.i64 v6, v41
;; @004e                               store notrap aligned v35, v45
;;                                     v51 = load.i32 notrap v55
;; @004e                               store notrap aligned v51, v21
;;                                     v62 = iconst.i64 4
;; @004e                               v46 = iadd.i64 v21, v62  ; v62 = 4
;; @004e                               store notrap aligned v46, v20
;; @004e                               jump block5
;;
;;                                 block3 cold:
;; @004e                               v48 = call fn0(v0, v15), stack_map=[i32 @ ss0+0]
;; @004e                               jump block5
;;
;;                                 block5:
;;                                     v49 = load.i32 notrap v55
;; @0052                               jump block1
;;
;;                                 block1:
;; @0052                               return v49
;; }
