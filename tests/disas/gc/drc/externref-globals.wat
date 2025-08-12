;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (global $x (mut externref) (ref.null extern))
  (func (export "get") (result externref)
    (global.get $x)
  )
  (func (export "set") (param externref)
    (global.set $x (local.get 0))
  )
)

;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v54 = iconst.i64 48
;; @0034                               v4 = iadd v0, v54  ; v54 = 48
;; @0034                               v5 = load.i32 notrap aligned v4
;;                                     v53 = iconst.i32 1
;; @0034                               v6 = band v5, v53  ; v53 = 1
;;                                     v52 = iconst.i32 0
;; @0034                               v7 = icmp eq v5, v52  ; v52 = 0
;; @0034                               v8 = uextend.i32 v7
;; @0034                               v9 = bor v6, v8
;; @0034                               brif v9, block4, block2
;;
;;                                 block2:
;; @0034                               v50 = load.i64 notrap aligned readonly can_move v0+8
;; @0034                               v11 = load.i64 notrap aligned readonly can_move v50+24
;; @0034                               v10 = uextend.i64 v5
;; @0034                               v12 = iadd v11, v10
;; @0034                               v13 = load.i32 notrap aligned v12
;; @0034                               v14 = iconst.i32 2
;; @0034                               v15 = band v13, v14  ; v14 = 2
;; @0034                               brif v15, block4, block3
;;
;;                                 block3:
;; @0034                               v17 = load.i64 notrap aligned readonly v0+32
;; @0034                               v18 = load.i32 notrap aligned v17
;; @0034                               v22 = iconst.i64 16
;; @0034                               v23 = iadd.i64 v12, v22  ; v22 = 16
;; @0034                               store notrap aligned v18, v23
;;                                     v55 = iconst.i32 2
;;                                     v56 = bor.i32 v13, v55  ; v55 = 2
;; @0034                               store notrap aligned v56, v12
;; @0034                               v32 = iconst.i64 8
;; @0034                               v33 = iadd.i64 v12, v32  ; v32 = 8
;; @0034                               v34 = load.i64 notrap aligned v33
;;                                     v43 = iconst.i64 1
;; @0034                               v35 = iadd v34, v43  ; v43 = 1
;; @0034                               store notrap aligned v35, v33
;; @0034                               store.i32 notrap aligned v5, v17
;; @0034                               jump block4
;;
;;                                 block4:
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return v5
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v55 = iconst.i64 48
;; @003b                               v4 = iadd v0, v55  ; v55 = 48
;; @003b                               v5 = load.i32 notrap aligned v4
;;                                     v54 = iconst.i32 1
;; @003b                               v6 = band v2, v54  ; v54 = 1
;;                                     v53 = iconst.i32 0
;; @003b                               v7 = icmp eq v2, v53  ; v53 = 0
;; @003b                               v8 = uextend.i32 v7
;; @003b                               v9 = bor v6, v8
;; @003b                               brif v9, block3, block2
;;
;;                                 block2:
;; @003b                               v44 = load.i64 notrap aligned readonly can_move v0+8
;; @003b                               v27 = load.i64 notrap aligned readonly can_move v44+24
;; @003b                               v10 = uextend.i64 v2
;; @003b                               v12 = iadd v27, v10
;; @003b                               v29 = iconst.i64 8
;; @003b                               v14 = iadd v12, v29  ; v29 = 8
;; @003b                               v15 = load.i64 notrap aligned v14
;;                                     v57 = iconst.i64 1
;; @003b                               v16 = iadd v15, v57  ; v57 = 1
;; @003b                               store notrap aligned v16, v14
;; @003b                               jump block3
;;
;;                                 block3:
;;                                     v69 = iadd.i64 v0, v55  ; v55 = 48
;; @003b                               store.i32 notrap aligned v2, v69
;;                                     v70 = iconst.i32 1
;;                                     v71 = band.i32 v5, v70  ; v70 = 1
;;                                     v72 = iconst.i32 0
;;                                     v73 = icmp.i32 eq v5, v72  ; v72 = 0
;; @003b                               v24 = uextend.i32 v73
;; @003b                               v25 = bor v71, v24
;; @003b                               brif v25, block7, block4
;;
;;                                 block4:
;;                                     v74 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v75 = load.i64 notrap aligned readonly can_move v74+24
;; @003b                               v26 = uextend.i64 v5
;; @003b                               v28 = iadd v75, v26
;;                                     v76 = iconst.i64 8
;; @003b                               v30 = iadd v28, v76  ; v76 = 8
;; @003b                               v31 = load.i64 notrap aligned v30
;;                                     v77 = iconst.i64 1
;;                                     v67 = icmp eq v31, v77  ; v77 = 1
;; @003b                               brif v67, block5, block6
;;
;;                                 block5 cold:
;; @003b                               call fn0(v0, v5)
;; @003b                               jump block7
;;
;;                                 block6:
;;                                     v43 = iconst.i64 -1
;; @003b                               v32 = iadd.i64 v31, v43  ; v43 = -1
;;                                     v78 = iadd.i64 v28, v76  ; v76 = 8
;; @003b                               store notrap aligned v32, v78
;; @003b                               jump block7
;;
;;                                 block7:
;; @003d                               jump block1
;;
;;                                 block1:
;; @003d                               return
;; }
