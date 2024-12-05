;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: f32):
;; @0034                               trapz v2, user16
;; @0034                               v9 = uextend.i64 v2
;; @0034                               v10 = iconst.i64 16
;; @0034                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 16
;;                                     v16 = iconst.i64 32
;; @0034                               v13 = uadd_overflow_trap v9, v16, user1  ; v16 = 32
;; @0034                               v8 = load.i64 notrap aligned readonly v0+48
;; @0034                               v14 = icmp ule v13, v8
;; @0034                               trapz v14, user1
;; @0034                               v6 = load.i64 notrap aligned readonly v0+40
;; @0034                               v15 = iadd v6, v11
;; @0034                               store notrap aligned little v3, v15
;; @0038                               jump block1
;;
;;                                 block1:
;; @0038                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @003f                               trapz v2, user16
;; @003f                               v9 = uextend.i64 v2
;; @003f                               v10 = iconst.i64 20
;; @003f                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 20
;;                                     v16 = iconst.i64 32
;; @003f                               v13 = uadd_overflow_trap v9, v16, user1  ; v16 = 32
;; @003f                               v8 = load.i64 notrap aligned readonly v0+48
;; @003f                               v14 = icmp ule v13, v8
;; @003f                               trapz v14, user1
;; @003f                               v6 = load.i64 notrap aligned readonly v0+40
;; @003f                               v15 = iadd v6, v11
;; @003f                               istore8 notrap aligned little v3, v15
;; @0043                               jump block1
;;
;;                                 block1:
;; @0043                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @004a                               trapz v2, user16
;; @004a                               v9 = uextend.i64 v2
;; @004a                               v10 = iconst.i64 24
;; @004a                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 24
;;                                     v78 = iconst.i64 32
;; @004a                               v13 = uadd_overflow_trap v9, v78, user1  ; v78 = 32
;; @004a                               v8 = load.i64 notrap aligned readonly v0+48
;; @004a                               v14 = icmp ule v13, v8
;; @004a                               trapz v14, user1
;; @004a                               v6 = load.i64 notrap aligned readonly v0+40
;; @004a                               v15 = iadd v6, v11
;; @004a                               v16 = load.i32 notrap aligned little v15
;; @004a                               v17 = iconst.i32 -2
;; @004a                               v18 = band v3, v17  ; v17 = -2
;;                                     v73 = iconst.i32 0
;; @004a                               v19 = icmp eq v18, v73  ; v73 = 0
;; @004a                               brif v19, block3, block2
;;
;;                                 block2:
;; @004a                               v24 = uextend.i64 v3
;; @004a                               v52 = iconst.i64 8
;; @004a                               v26 = uadd_overflow_trap v24, v52, user1  ; v52 = 8
;; @004a                               v28 = uadd_overflow_trap v26, v52, user1  ; v52 = 8
;; @004a                               v29 = icmp ule v28, v8
;; @004a                               trapz v29, user1
;; @004a                               v30 = iadd.i64 v6, v26
;; @004a                               v31 = load.i64 notrap aligned v30
;; @004a                               trapz v29, user1
;;                                     v74 = iconst.i64 1
;; @004a                               v32 = iadd v31, v74  ; v74 = 1
;; @004a                               store notrap aligned v32, v30
;; @004a                               jump block3
;;
;;                                 block3:
;; @004a                               store.i32 notrap aligned little v3, v15
;;                                     v79 = iconst.i32 -2
;;                                     v80 = band.i32 v16, v79  ; v79 = -2
;;                                     v81 = iconst.i32 0
;;                                     v82 = icmp eq v80, v81  ; v81 = 0
;; @004a                               brif v82, block7, block4
;;
;;                                 block4:
;; @004a                               v51 = uextend.i64 v16
;;                                     v83 = iconst.i64 8
;; @004a                               v53 = uadd_overflow_trap v51, v83, user1  ; v83 = 8
;; @004a                               v55 = uadd_overflow_trap v53, v83, user1  ; v83 = 8
;; @004a                               v56 = icmp ule v55, v8
;; @004a                               trapz v56, user1
;; @004a                               v57 = iadd.i64 v6, v53
;; @004a                               v58 = load.i64 notrap aligned v57
;;                                     v76 = iconst.i64 -1
;; @004a                               v59 = iadd v58, v76  ; v76 = -1
;;                                     v77 = iconst.i64 0
;; @004a                               v60 = icmp eq v59, v77  ; v77 = 0
;; @004a                               brif v60, block5, block6
;;
;;                                 block5 cold:
;; @004a                               call fn0(v0, v16)
;; @004a                               jump block7
;;
;;                                 block6:
;; @004a                               trapz.i8 v56, user1
;;                                     v84 = iadd.i64 v58, v76  ; v76 = -1
;; @004a                               store notrap aligned v84, v57
;; @004a                               jump block7
;;
;;                                 block7:
;; @004e                               jump block1
;;
;;                                 block1:
;; @004e                               return
;; }
