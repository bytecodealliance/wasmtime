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
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @004a                               trapz v2, user16
;; @004a                               v9 = uextend.i64 v2
;; @004a                               v10 = iconst.i64 24
;; @004a                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 24
;;                                     v82 = iconst.i64 32
;; @004a                               v13 = uadd_overflow_trap v9, v82, user1  ; v82 = 32
;; @004a                               v8 = load.i64 notrap aligned readonly v0+48
;; @004a                               v14 = icmp ule v13, v8
;; @004a                               trapz v14, user1
;; @004a                               v6 = load.i64 notrap aligned readonly v0+40
;; @004a                               v15 = iadd v6, v11
;; @004a                               v16 = load.i32 notrap aligned little v15
;;                                     v75 = iconst.i32 1
;; @004a                               v17 = band v3, v75  ; v75 = 1
;;                                     v76 = iconst.i32 0
;; @004a                               v18 = icmp eq v3, v76  ; v76 = 0
;; @004a                               v19 = uextend.i32 v18
;; @004a                               v20 = bor v17, v19
;; @004a                               brif v20, block3, block2
;;
;;                                 block2:
;; @004a                               v25 = uextend.i64 v3
;; @004a                               v54 = iconst.i64 8
;; @004a                               v27 = uadd_overflow_trap v25, v54, user1  ; v54 = 8
;; @004a                               v29 = uadd_overflow_trap v27, v54, user1  ; v54 = 8
;; @004a                               v30 = icmp ule v29, v8
;; @004a                               trapz v30, user1
;; @004a                               v31 = iadd.i64 v6, v27
;; @004a                               v32 = load.i64 notrap aligned v31
;;                                     v77 = iconst.i64 1
;; @004a                               v33 = iadd v32, v77  ; v77 = 1
;; @004a                               store notrap aligned v33, v31
;; @004a                               jump block3
;;
;;                                 block3:
;; @004a                               store.i32 notrap aligned little v3, v15
;;                                     v83 = iconst.i32 1
;;                                     v84 = band.i32 v16, v83  ; v83 = 1
;;                                     v85 = iconst.i32 0
;;                                     v86 = icmp.i32 eq v16, v85  ; v85 = 0
;; @004a                               v47 = uextend.i32 v86
;; @004a                               v48 = bor v84, v47
;; @004a                               brif v48, block7, block4
;;
;;                                 block4:
;; @004a                               v53 = uextend.i64 v16
;;                                     v87 = iconst.i64 8
;; @004a                               v55 = uadd_overflow_trap v53, v87, user1  ; v87 = 8
;; @004a                               v57 = uadd_overflow_trap v55, v87, user1  ; v87 = 8
;; @004a                               v58 = icmp ule v57, v8
;; @004a                               trapz v58, user1
;; @004a                               v59 = iadd.i64 v6, v55
;; @004a                               v60 = load.i64 notrap aligned v59
;;                                     v80 = iconst.i64 -1
;; @004a                               v61 = iadd v60, v80  ; v80 = -1
;;                                     v81 = iconst.i64 0
;; @004a                               v62 = icmp eq v61, v81  ; v81 = 0
;; @004a                               brif v62, block5, block6
;;
;;                                 block5 cold:
;; @004a                               call fn0(v0, v16)
;; @004a                               jump block7
;;
;;                                 block6:
;;                                     v88 = iadd.i64 v60, v80  ; v80 = -1
;; @004a                               store notrap aligned v88, v59
;; @004a                               jump block7
;;
;;                                 block7:
;; @004e                               jump block1
;;
;;                                 block1:
;; @004e                               return
;; }
