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
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:45 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v92 = iconst.i64 48
;; @0034                               v4 = iadd v0, v92  ; v92 = 48
;; @0034                               v5 = load.i32 notrap aligned v4
;;                                     v91 = stack_addr.i64 ss0
;;                                     store notrap v5, v91
;;                                     v89 = iconst.i32 1
;; @0034                               v6 = band v5, v89  ; v89 = 1
;;                                     v87 = iconst.i32 0
;; @0034                               v7 = icmp eq v5, v87  ; v87 = 0
;; @0034                               v8 = uextend.i32 v7
;; @0034                               v9 = bor v6, v8
;; @0034                               brif v9, block4, block2
;;
;;                                 block2:
;; @0034                               v84 = load.i64 notrap aligned readonly can_move v0+8
;; @0034                               v11 = load.i64 notrap aligned readonly can_move v84+32
;; @0034                               v10 = uextend.i64 v5
;; @0034                               v12 = iadd v11, v10
;; @0034                               v13 = load.i32 user2 v12
;; @0034                               v14 = iconst.i32 2
;; @0034                               v15 = band v13, v14  ; v14 = 2
;; @0034                               brif v15, block4, block3
;;
;;                                 block3:
;; @0034                               v17 = load.i64 notrap aligned readonly can_move v0+32
;; @0034                               v18 = load.i32 user2 v17
;; @0034                               v22 = iconst.i64 16
;; @0034                               v23 = iadd.i64 v12, v22  ; v22 = 16
;; @0034                               store user2 v18, v23
;;                                     v63 = load.i32 notrap v91
;;                                     v93 = iconst.i32 2
;;                                     v94 = bor.i32 v13, v93  ; v93 = 2
;; @0034                               v26 = uextend.i64 v63
;; @0034                               v28 = iadd.i64 v11, v26
;; @0034                               store user2 v94, v28
;;                                     v62 = load.i32 notrap v91
;; @0034                               v29 = uextend.i64 v62
;; @0034                               v31 = iadd.i64 v11, v29
;; @0034                               v32 = iconst.i64 8
;; @0034                               v33 = iadd v31, v32  ; v32 = 8
;; @0034                               v34 = load.i64 user2 v33
;;                                     v74 = iconst.i64 1
;; @0034                               v35 = iadd v34, v74  ; v74 = 1
;; @0034                               store user2 v35, v33
;;                                     v60 = load.i32 notrap v91
;; @0034                               store user2 v60, v17
;; @0034                               v43 = load.i32 notrap aligned v17+4
;;                                     v95 = iconst.i32 1
;;                                     v96 = iadd v43, v95  ; v95 = 1
;; @0034                               store notrap aligned v96, v17+4
;; @0034                               v52 = load.i32 notrap aligned v17+8
;; @0034                               v53 = iadd v52, v52
;; @0034                               v54 = iconst.i32 1024
;; @0034                               v55 = umax v53, v54  ; v54 = 1024
;; @0034                               v56 = icmp uge v96, v55
;; @0034                               brif v56, block5, block6
;;
;;                                 block5 cold:
;; @0034                               v58 = call fn0(v0), stack_map=[i32 @ ss0+0]
;; @0034                               jump block6
;;
;;                                 block6:
;; @0034                               jump block4
;;
;;                                 block4:
;;                                     v59 = load.i32 notrap v91
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return v59
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) tail {
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
;; @003b                               v51 = load.i64 notrap aligned readonly can_move v0+8
;; @003b                               v11 = load.i64 notrap aligned readonly can_move v51+32
;; @003b                               v10 = uextend.i64 v2
;; @003b                               v12 = iadd v11, v10
;; @003b                               v13 = iconst.i64 8
;; @003b                               v14 = iadd v12, v13  ; v13 = 8
;; @003b                               v15 = load.i64 user2 v14
;;                                     v50 = iconst.i64 1
;; @003b                               v16 = iadd v15, v50  ; v50 = 1
;; @003b                               store user2 v16, v14
;; @003b                               jump block3
;;
;;                                 block3:
;;                                     v68 = iadd.i64 v0, v55  ; v55 = 48
;; @003b                               store.i32 notrap aligned v2, v68
;;                                     v69 = iconst.i32 1
;;                                     v70 = band.i32 v5, v69  ; v69 = 1
;;                                     v71 = iconst.i32 0
;;                                     v72 = icmp.i32 eq v5, v71  ; v71 = 0
;; @003b                               v24 = uextend.i32 v72
;; @003b                               v25 = bor v70, v24
;; @003b                               brif v25, block7, block4
;;
;;                                 block4:
;;                                     v73 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v74 = load.i64 notrap aligned readonly can_move v73+32
;; @003b                               v26 = uextend.i64 v5
;; @003b                               v28 = iadd v74, v26
;;                                     v75 = iconst.i64 8
;; @003b                               v30 = iadd v28, v75  ; v75 = 8
;; @003b                               v31 = load.i64 user2 v30
;;                                     v76 = iconst.i64 1
;;                                     v66 = icmp eq v31, v76  ; v76 = 1
;; @003b                               brif v66, block5, block6
;;
;;                                 block5 cold:
;; @003b                               call fn0(v0, v5)
;; @003b                               jump block7
;;
;;                                 block6:
;;                                     v43 = iconst.i64 -1
;; @003b                               v32 = iadd.i64 v31, v43  ; v43 = -1
;;                                     v77 = iadd.i64 v28, v75  ; v75 = 8
;; @003b                               store user2 v32, v77
;; @003b                               jump block7
;;
;;                                 block7:
;; @003d                               jump block1
;;
;;                                 block1:
;; @003d                               return
;; }
