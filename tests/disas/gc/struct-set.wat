;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (func (param (ref null $ty) f32)
    (struct.set $ty 0 (local.get 0) (local.get 1))
  )

  (func (param (ref null $ty) i32)
    (struct.set $ty 1 (local.get 0) (local.get 1))
  )

  (func (param (ref null $ty) anyref)
    (struct.set $ty 2 (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i32, f32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: f32):
;; @0034                               trapz v2, null_reference
;; @0034                               v8 = uextend.i64 v2
;; @0034                               v9 = iconst.i64 16
;; @0034                               v10 = uadd_overflow_trap v8, v9, user65535  ; v9 = 16
;;                                     v15 = iconst.i64 32
;; @0034                               v12 = uadd_overflow_trap v8, v15, user65535  ; v15 = 32
;; @0034                               v7 = load.i64 notrap aligned readonly v0+48
;; @0034                               v13 = icmp ule v12, v7
;; @0034                               trapz v13, user65535
;; @0034                               v6 = load.i64 notrap aligned readonly v0+40
;; @0034                               v14 = iadd v6, v10
;; @0034                               store notrap aligned little v3, v14
;; @0038                               jump block1
;;
;;                                 block1:
;; @0038                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @003f                               trapz v2, null_reference
;; @003f                               v8 = uextend.i64 v2
;; @003f                               v9 = iconst.i64 20
;; @003f                               v10 = uadd_overflow_trap v8, v9, user65535  ; v9 = 20
;;                                     v15 = iconst.i64 32
;; @003f                               v12 = uadd_overflow_trap v8, v15, user65535  ; v15 = 32
;; @003f                               v7 = load.i64 notrap aligned readonly v0+48
;; @003f                               v13 = icmp ule v12, v7
;; @003f                               trapz v13, user65535
;; @003f                               v6 = load.i64 notrap aligned readonly v0+40
;; @003f                               v14 = iadd v6, v10
;; @003f                               istore8 notrap aligned little v3, v14
;; @0043                               jump block1
;;
;;                                 block1:
;; @0043                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext) system_v
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @004a                               trapz v2, null_reference
;; @004a                               v8 = uextend.i64 v2
;; @004a                               v9 = iconst.i64 24
;; @004a                               v10 = uadd_overflow_trap v8, v9, user65535  ; v9 = 24
;;                                     v73 = iconst.i64 32
;; @004a                               v12 = uadd_overflow_trap v8, v73, user65535  ; v73 = 32
;; @004a                               v7 = load.i64 notrap aligned readonly v0+48
;; @004a                               v13 = icmp ule v12, v7
;; @004a                               trapz v13, user65535
;; @004a                               v6 = load.i64 notrap aligned readonly v0+40
;; @004a                               v14 = iadd v6, v10
;; @004a                               v15 = load.i32 notrap aligned little v14
;; @004a                               v16 = iconst.i32 -2
;; @004a                               v17 = band v3, v16  ; v16 = -2
;;                                     v68 = iconst.i32 0
;; @004a                               v18 = icmp eq v17, v68  ; v68 = 0
;; @004a                               brif v18, block3, block2
;;
;;                                 block2:
;; @004a                               v22 = uextend.i64 v3
;; @004a                               v48 = iconst.i64 8
;; @004a                               v24 = uadd_overflow_trap v22, v48, user65535  ; v48 = 8
;; @004a                               v26 = uadd_overflow_trap v24, v48, user65535  ; v48 = 8
;; @004a                               v27 = icmp ule v26, v7
;; @004a                               trapz v27, user65535
;; @004a                               v28 = iadd.i64 v6, v24
;; @004a                               v29 = load.i64 notrap aligned v28
;; @004a                               trapz v27, user65535
;;                                     v69 = iconst.i64 1
;; @004a                               v30 = iadd v29, v69  ; v69 = 1
;; @004a                               store notrap aligned v30, v28
;; @004a                               jump block3
;;
;;                                 block3:
;; @004a                               store.i32 notrap aligned little v3, v14
;;                                     v74 = iconst.i32 -2
;;                                     v75 = band.i32 v15, v74  ; v74 = -2
;;                                     v76 = iconst.i32 0
;;                                     v77 = icmp eq v75, v76  ; v76 = 0
;; @004a                               brif v77, block7, block4
;;
;;                                 block4:
;; @004a                               v47 = uextend.i64 v15
;;                                     v78 = iconst.i64 8
;; @004a                               v49 = uadd_overflow_trap v47, v78, user65535  ; v78 = 8
;; @004a                               v51 = uadd_overflow_trap v49, v78, user65535  ; v78 = 8
;; @004a                               v52 = icmp ule v51, v7
;; @004a                               trapz v52, user65535
;; @004a                               v53 = iadd.i64 v6, v49
;; @004a                               v54 = load.i64 notrap aligned v53
;;                                     v71 = iconst.i64 -1
;; @004a                               v55 = iadd v54, v71  ; v71 = -1
;;                                     v72 = iconst.i64 0
;; @004a                               v56 = icmp eq v55, v72  ; v72 = 0
;; @004a                               brif v56, block5, block6
;;
;;                                 block5 cold:
;; @004a                               call fn0(v0, v15)
;; @004a                               jump block7
;;
;;                                 block6:
;; @004a                               trapz.i8 v52, user65535
;;                                     v79 = iadd.i64 v54, v71  ; v71 = -1
;; @004a                               store notrap aligned v79, v53
;; @004a                               jump block7
;;
;;                                 block7:
;; @004e                               jump block1
;;
;;                                 block1:
;; @004e                               return
;; }
