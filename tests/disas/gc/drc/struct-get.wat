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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0033                               trapz v2, user16
;; @0033                               v9 = uextend.i64 v2
;; @0033                               v10 = iconst.i64 16
;; @0033                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 16
;;                                     v17 = iconst.i64 32
;; @0033                               v13 = uadd_overflow_trap v9, v17, user1  ; v17 = 32
;; @0033                               v8 = load.i64 notrap aligned readonly v0+48
;; @0033                               v14 = icmp ule v13, v8
;; @0033                               trapz v14, user1
;; @0033                               v6 = load.i64 notrap aligned readonly v0+40
;; @0033                               v15 = iadd v6, v11
;; @0033                               v16 = load.f32 notrap aligned little v15
;; @0037                               jump block1
;;
;;                                 block1:
;; @0037                               return v16
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003c                               trapz v2, user16
;; @003c                               v9 = uextend.i64 v2
;; @003c                               v10 = iconst.i64 20
;; @003c                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 20
;;                                     v18 = iconst.i64 32
;; @003c                               v13 = uadd_overflow_trap v9, v18, user1  ; v18 = 32
;; @003c                               v8 = load.i64 notrap aligned readonly v0+48
;; @003c                               v14 = icmp ule v13, v8
;; @003c                               trapz v14, user1
;; @003c                               v6 = load.i64 notrap aligned readonly v0+40
;; @003c                               v15 = iadd v6, v11
;; @003c                               v16 = load.i8 notrap aligned little v15
;; @0040                               jump block1
;;
;;                                 block1:
;; @003c                               v17 = sextend.i32 v16
;; @0040                               return v17
;; }
;;
;; function u0:2(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0045                               trapz v2, user16
;; @0045                               v9 = uextend.i64 v2
;; @0045                               v10 = iconst.i64 20
;; @0045                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 20
;;                                     v18 = iconst.i64 32
;; @0045                               v13 = uadd_overflow_trap v9, v18, user1  ; v18 = 32
;; @0045                               v8 = load.i64 notrap aligned readonly v0+48
;; @0045                               v14 = icmp ule v13, v8
;; @0045                               trapz v14, user1
;; @0045                               v6 = load.i64 notrap aligned readonly v0+40
;; @0045                               v15 = iadd v6, v11
;; @0045                               v16 = load.i8 notrap aligned little v15
;; @0049                               jump block1
;;
;;                                 block1:
;; @0045                               v17 = uextend.i32 v16
;; @0049                               return v17
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext) -> i64 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @004e                               trapz v2, user16
;; @004e                               v9 = uextend.i64 v2
;; @004e                               v10 = iconst.i64 24
;; @004e                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 24
;;                                     v68 = iconst.i64 32
;; @004e                               v13 = uadd_overflow_trap v9, v68, user1  ; v68 = 32
;; @004e                               v8 = load.i64 notrap aligned readonly v0+48
;; @004e                               v14 = icmp ule v13, v8
;; @004e                               trapz v14, user1
;; @004e                               v6 = load.i64 notrap aligned readonly v0+40
;; @004e                               v15 = iadd v6, v11
;; @004e                               v16 = load.i32 notrap aligned little v15
;;                                     v58 = stack_addr.i64 ss0
;;                                     store notrap v16, v58
;; @004e                               v17 = iconst.i32 -2
;; @004e                               v18 = band v16, v17  ; v17 = -2
;;                                     v60 = iconst.i32 0
;; @004e                               v19 = icmp eq v18, v60  ; v60 = 0
;; @004e                               brif v19, block5, block2
;;
;;                                 block2:
;; @004e                               v21 = load.i64 notrap aligned readonly v0+56
;; @004e                               v22 = load.i64 notrap aligned v21
;; @004e                               v23 = load.i64 notrap aligned v21+8
;; @004e                               v24 = icmp eq v22, v23
;; @004e                               brif v24, block3, block4
;;
;;                                 block4:
;; @004e                               v29 = uextend.i64 v16
;; @004e                               v30 = iconst.i64 8
;; @004e                               v31 = uadd_overflow_trap v29, v30, user1  ; v30 = 8
;; @004e                               v33 = uadd_overflow_trap v31, v30, user1  ; v30 = 8
;; @004e                               v34 = icmp ule v33, v8
;; @004e                               trapz v34, user1
;; @004e                               v35 = iadd.i64 v6, v31
;; @004e                               v36 = load.i64 notrap aligned v35
;;                                     v62 = iconst.i64 1
;; @004e                               v37 = iadd v36, v62  ; v62 = 1
;; @004e                               store notrap aligned v37, v35
;;                                     v54 = load.i32 notrap v58
;; @004e                               store notrap aligned v54, v22
;;                                     v65 = iconst.i64 4
;; @004e                               v49 = iadd.i64 v22, v65  ; v65 = 4
;; @004e                               store notrap aligned v49, v21
;; @004e                               jump block5
;;
;;                                 block3 cold:
;; @004e                               v51 = call fn0(v0, v16), stack_map=[i32 @ ss0+0]
;; @004e                               jump block5
;;
;;                                 block5:
;;                                     v52 = load.i32 notrap v58
;; @0052                               jump block1
;;
;;                                 block1:
;; @0052                               return v52
;; }
