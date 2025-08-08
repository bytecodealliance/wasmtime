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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: f32):
;; @0034                               trapz v2, user16
;; @0034                               v9 = load.i64 notrap aligned readonly can_move v0+8
;; @0034                               v5 = load.i64 notrap aligned readonly can_move v9+24
;; @0034                               v4 = uextend.i64 v2
;; @0034                               v6 = iadd v5, v4
;; @0034                               v7 = iconst.i64 24
;; @0034                               v8 = iadd v6, v7  ; v7 = 24
;; @0034                               store notrap aligned little v3, v8
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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @003f                               trapz v2, user16
;; @003f                               v9 = load.i64 notrap aligned readonly can_move v0+8
;; @003f                               v5 = load.i64 notrap aligned readonly can_move v9+24
;; @003f                               v4 = uextend.i64 v2
;; @003f                               v6 = iadd v5, v4
;; @003f                               v7 = iconst.i64 28
;; @003f                               v8 = iadd v6, v7  ; v7 = 28
;; @003f                               istore8 notrap aligned little v3, v8
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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1610612736:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @004a                               trapz v2, user16
;; @004a                               v59 = load.i64 notrap aligned readonly can_move v0+8
;; @004a                               v5 = load.i64 notrap aligned readonly can_move v59+24
;; @004a                               v4 = uextend.i64 v2
;; @004a                               v6 = iadd v5, v4
;; @004a                               v7 = iconst.i64 32
;; @004a                               v8 = iadd v6, v7  ; v7 = 32
;; @004a                               v9 = load.i32 notrap aligned little v8
;;                                     v58 = iconst.i32 1
;; @004a                               v10 = band v3, v58  ; v58 = 1
;;                                     v57 = iconst.i32 0
;; @004a                               v11 = icmp eq v3, v57  ; v57 = 0
;; @004a                               v12 = uextend.i32 v11
;; @004a                               v13 = bor v10, v12
;; @004a                               brif v13, block3, block2
;;
;;                                 block2:
;; @004a                               v14 = uextend.i64 v3
;; @004a                               v16 = iadd.i64 v5, v14
;; @004a                               v33 = iconst.i64 8
;; @004a                               v18 = iadd v16, v33  ; v33 = 8
;; @004a                               v19 = load.i64 notrap aligned v18
;;                                     v62 = iconst.i64 1
;; @004a                               v20 = iadd v19, v62  ; v62 = 1
;; @004a                               store notrap aligned v20, v18
;; @004a                               jump block3
;;
;;                                 block3:
;;                                     v74 = iadd.i64 v6, v7  ; v7 = 32
;; @004a                               store.i32 notrap aligned little v3, v74
;;                                     v75 = iconst.i32 1
;;                                     v76 = band.i32 v9, v75  ; v75 = 1
;;                                     v77 = iconst.i32 0
;;                                     v78 = icmp.i32 eq v9, v77  ; v77 = 0
;; @004a                               v28 = uextend.i32 v78
;; @004a                               v29 = bor v76, v28
;; @004a                               brif v29, block7, block4
;;
;;                                 block4:
;; @004a                               v30 = uextend.i64 v9
;; @004a                               v32 = iadd.i64 v5, v30
;;                                     v79 = iconst.i64 8
;; @004a                               v34 = iadd v32, v79  ; v79 = 8
;; @004a                               v35 = load.i64 notrap aligned v34
;;                                     v80 = iconst.i64 1
;;                                     v72 = icmp eq v35, v80  ; v80 = 1
;; @004a                               brif v72, block5, block6
;;
;;                                 block5 cold:
;; @004a                               call fn0(v0, v9)
;; @004a                               jump block7
;;
;;                                 block6:
;;                                     v47 = iconst.i64 -1
;; @004a                               v36 = iadd.i64 v35, v47  ; v47 = -1
;;                                     v81 = iadd.i64 v32, v79  ; v79 = 8
;; @004a                               store notrap aligned v36, v81
;; @004a                               jump block7
;;
;;                                 block7:
;; @004e                               jump block1
;;
;;                                 block1:
;; @004e                               return
;; }
