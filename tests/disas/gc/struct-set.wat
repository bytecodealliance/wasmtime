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
;; @0034                               v7 = uextend.i64 v2
;; @0034                               v8 = iconst.i64 16
;; @0034                               v9 = uadd_overflow_trap v7, v8, user65535  ; v8 = 16
;; @0034                               v10 = iconst.i64 4
;; @0034                               v11 = uadd_overflow_trap v9, v10, user65535  ; v10 = 4
;; @0034                               v6 = load.i64 notrap aligned readonly v0+48
;; @0034                               v12 = icmp ult v11, v6
;; @0034                               trapz v12, user65535
;; @0034                               v5 = load.i64 notrap aligned readonly v0+40
;; @0034                               v13 = iadd v5, v9
;; @0034                               store notrap aligned little v3, v13
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
;; @003f                               v7 = uextend.i64 v2
;; @003f                               v8 = iconst.i64 20
;; @003f                               v9 = uadd_overflow_trap v7, v8, user65535  ; v8 = 20
;; @003f                               v10 = iconst.i64 1
;; @003f                               v11 = uadd_overflow_trap v9, v10, user65535  ; v10 = 1
;; @003f                               v6 = load.i64 notrap aligned readonly v0+48
;; @003f                               v12 = icmp ult v11, v6
;; @003f                               trapz v12, user65535
;; @003f                               v5 = load.i64 notrap aligned readonly v0+40
;; @003f                               v13 = iadd v5, v9
;; @003f                               istore8 notrap aligned little v3, v13
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
;; @004a                               v7 = uextend.i64 v2
;; @004a                               v8 = iconst.i64 24
;; @004a                               v9 = uadd_overflow_trap v7, v8, user65535  ; v8 = 24
;; @004a                               v10 = iconst.i64 4
;; @004a                               v11 = uadd_overflow_trap v9, v10, user65535  ; v10 = 4
;; @004a                               v6 = load.i64 notrap aligned readonly v0+48
;; @004a                               v12 = icmp ult v11, v6
;; @004a                               trapz v12, user65535
;; @004a                               v5 = load.i64 notrap aligned readonly v0+40
;; @004a                               v13 = iadd v5, v9
;; @004a                               v14 = load.i32 notrap aligned little v13
;; @004a                               v15 = iconst.i32 -2
;; @004a                               v16 = band v3, v15  ; v15 = -2
;;                                     v67 = iconst.i32 0
;; @004a                               v17 = icmp eq v16, v67  ; v67 = 0
;; @004a                               brif v17, block3, block2
;;
;;                                 block2:
;; @004a                               v21 = uextend.i64 v3
;; @004a                               v47 = iconst.i64 8
;; @004a                               v23 = uadd_overflow_trap v21, v47, user65535  ; v47 = 8
;; @004a                               v25 = uadd_overflow_trap v23, v47, user65535  ; v47 = 8
;; @004a                               v26 = icmp ult v25, v6
;; @004a                               trapz v26, user65535
;; @004a                               v27 = iadd.i64 v5, v23
;; @004a                               v28 = load.i64 notrap aligned v27
;; @004a                               trapz v26, user65535
;;                                     v68 = iconst.i64 1
;; @004a                               v29 = iadd v28, v68  ; v68 = 1
;; @004a                               store notrap aligned v29, v27
;; @004a                               jump block3
;;
;;                                 block3:
;; @004a                               store.i32 notrap aligned little v3, v13
;;                                     v72 = iconst.i32 -2
;;                                     v73 = band.i32 v14, v72  ; v72 = -2
;;                                     v74 = iconst.i32 0
;;                                     v75 = icmp eq v73, v74  ; v74 = 0
;; @004a                               brif v75, block7, block4
;;
;;                                 block4:
;; @004a                               v46 = uextend.i64 v14
;;                                     v76 = iconst.i64 8
;; @004a                               v48 = uadd_overflow_trap v46, v76, user65535  ; v76 = 8
;; @004a                               v50 = uadd_overflow_trap v48, v76, user65535  ; v76 = 8
;; @004a                               v51 = icmp ult v50, v6
;; @004a                               trapz v51, user65535
;; @004a                               v52 = iadd.i64 v5, v48
;; @004a                               v53 = load.i64 notrap aligned v52
;;                                     v70 = iconst.i64 -1
;; @004a                               v54 = iadd v53, v70  ; v70 = -1
;;                                     v71 = iconst.i64 0
;; @004a                               v55 = icmp eq v54, v71  ; v71 = 0
;; @004a                               brif v55, block5, block6
;;
;;                                 block5 cold:
;; @004a                               call fn0(v0, v14)
;; @004a                               jump block7
;;
;;                                 block6:
;; @004a                               trapz.i8 v51, user65535
;;                                     v77 = iadd.i64 v53, v70  ; v70 = -1
;; @004a                               store notrap aligned v77, v52
;; @004a                               jump block7
;;
;;                                 block7:
;; @004e                               jump block1
;;
;;                                 block1:
;; @004e                               return
;; }
