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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: f32):
;; @0034                               trapz v2, user16
;; @0034                               v9 = load.i64 notrap aligned readonly can_move v0+8
;; @0034                               v5 = load.i64 notrap aligned readonly can_move v9+32
;; @0034                               v4 = uextend.i64 v2
;; @0034                               v6 = iadd v5, v4
;; @0034                               v7 = iconst.i64 24
;; @0034                               v8 = iadd v6, v7  ; v7 = 24
;; @0034                               store user2 little region0 v3, v8
;; @0038                               jump block1
;;
;;                                 block1:
;; @0038                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @003f                               trapz v2, user16
;; @003f                               v9 = load.i64 notrap aligned readonly can_move v0+8
;; @003f                               v5 = load.i64 notrap aligned readonly can_move v9+32
;; @003f                               v4 = uextend.i64 v2
;; @003f                               v6 = iadd v5, v4
;; @003f                               v7 = iconst.i64 28
;; @003f                               v8 = iadd v6, v7  ; v7 = 28
;; @003f                               istore8 user2 little region0 v3, v8
;; @0043                               jump block1
;;
;;                                 block1:
;; @0043                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32) tail {
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u805306368:22 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @004a                               trapz v2, user16
;; @004a                               v59 = load.i64 notrap aligned readonly can_move v0+8
;; @004a                               v5 = load.i64 notrap aligned readonly can_move v59+32
;; @004a                               v4 = uextend.i64 v2
;; @004a                               v6 = iadd v5, v4
;; @004a                               v7 = iconst.i64 32
;; @004a                               v8 = iadd v6, v7  ; v7 = 32
;; @004a                               v9 = load.i32 user2 little region0 v8
;;                                     v58 = iconst.i32 1
;; @004a                               v10 = band v3, v58  ; v58 = 1
;; @004a                               v11 = iconst.i32 0
;; @004a                               v12 = icmp eq v3, v11  ; v11 = 0
;; @004a                               v13 = uextend.i32 v12
;; @004a                               v14 = bor v10, v13
;; @004a                               brif v14, block3, block2
;;
;;                                 block2:
;; @004a                               v15 = uextend.i64 v3
;; @004a                               v17 = iadd.i64 v5, v15
;; @004a                               v18 = iconst.i64 8
;; @004a                               v19 = iadd v17, v18  ; v18 = 8
;; @004a                               v20 = load.i64 user2 region0 v19
;; @004a                               v21 = iconst.i64 1
;; @004a                               v22 = iadd v20, v21  ; v21 = 1
;; @004a                               store user2 region0 v22, v19
;; @004a                               jump block3
;;
;;                                 block3:
;;                                     v73 = iadd.i64 v6, v7  ; v7 = 32
;; @004a                               store.i32 user2 little region0 v3, v73
;;                                     v74 = iconst.i32 1
;;                                     v75 = band.i32 v9, v74  ; v74 = 1
;;                                     v76 = iconst.i32 0
;;                                     v77 = icmp.i32 eq v9, v76  ; v76 = 0
;; @004a                               v31 = uextend.i32 v77
;; @004a                               v32 = bor v75, v31
;; @004a                               brif v32, block7, block4
;;
;;                                 block4:
;; @004a                               v33 = uextend.i64 v9
;; @004a                               v35 = iadd.i64 v5, v33
;;                                     v78 = iconst.i64 8
;; @004a                               v37 = iadd v35, v78  ; v78 = 8
;; @004a                               v38 = load.i64 user2 region0 v37
;;                                     v79 = iconst.i64 1
;;                                     v71 = icmp eq v38, v79  ; v79 = 1
;; @004a                               brif v71, block5, block6
;;
;;                                 block5 cold:
;; @004a                               call fn0(v0, v9)
;; @004a                               jump block7
;;
;;                                 block6:
;; @004a                               v39 = iconst.i64 -1
;; @004a                               v40 = iadd.i64 v38, v39  ; v39 = -1
;;                                     v80 = iadd.i64 v35, v78  ; v78 = 8
;; @004a                               store user2 region0 v40, v80
;; @004a                               jump block7
;;
;;                                 block7:
;; @004e                               jump block1
;;
;;                                 block1:
;; @004e                               return
;; }
