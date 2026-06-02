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
;;     region0 = 2147483648 "GcHeap"
;;     region1 = 32 "VMContext+0x20"
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
;; @0034                               v7 = iconst.i32 0
;; @0034                               v8 = icmp eq v5, v7  ; v7 = 0
;; @0034                               v9 = uextend.i32 v8
;; @0034                               v10 = bor v6, v9
;; @0034                               brif v10, block4, block2
;;
;;                                 block2:
;; @0034                               v85 = load.i64 notrap aligned readonly can_move v0+8
;; @0034                               v12 = load.i64 notrap aligned readonly can_move v85+32
;; @0034                               v11 = uextend.i64 v5
;; @0034                               v13 = iadd v12, v11
;; @0034                               v14 = load.i32 user2 region0 v13
;; @0034                               v15 = iconst.i32 2
;; @0034                               v16 = band v14, v15  ; v15 = 2
;; @0034                               brif v16, block4, block3
;;
;;                                 block3:
;; @0034                               v18 = load.i64 notrap aligned readonly can_move region1 v0+32
;; @0034                               v19 = load.i32 user2 region0 v18
;; @0034                               v23 = iconst.i64 16
;; @0034                               v24 = iadd.i64 v13, v23  ; v23 = 16
;; @0034                               store user2 region0 v19, v24
;;                                     v93 = iconst.i32 2
;;                                     v94 = bor.i32 v14, v93  ; v93 = 2
;; @0034                               store user2 region0 v94, v13
;; @0034                               v33 = iconst.i64 8
;; @0034                               v34 = iadd.i64 v13, v33  ; v33 = 8
;; @0034                               v35 = load.i64 user2 region0 v34
;;                                     v75 = iconst.i64 1
;; @0034                               v36 = iadd v35, v75  ; v75 = 1
;; @0034                               store user2 region0 v36, v34
;; @0034                               store.i32 user2 region0 v5, v18
;; @0034                               v44 = load.i32 notrap aligned v18+4
;;                                     v95 = iconst.i32 1
;;                                     v96 = iadd v44, v95  ; v95 = 1
;; @0034                               store notrap aligned v96, v18+4
;; @0034                               v53 = load.i32 notrap aligned v18+8
;; @0034                               v54 = iadd v53, v53
;; @0034                               v55 = iconst.i32 1024
;; @0034                               v56 = umax v54, v55  ; v55 = 1024
;; @0034                               v57 = icmp uge v96, v56
;; @0034                               brif v57, block5, block6
;;
;;                                 block5 cold:
;; @0034                               v59 = call fn0(v0), stack_map=[i32 @ ss0+0]
;; @0034                               jump block6
;;
;;                                 block6:
;; @0034                               jump block4
;;
;;                                 block4:
;;                                     v60 = load.i32 notrap v91
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return v60
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v55 = iconst.i64 48
;; @003b                               v4 = iadd v0, v55  ; v55 = 48
;; @003b                               v5 = load.i32 notrap aligned v4
;;                                     v54 = iconst.i32 1
;; @003b                               v6 = band v2, v54  ; v54 = 1
;; @003b                               v7 = iconst.i32 0
;; @003b                               v8 = icmp eq v2, v7  ; v7 = 0
;; @003b                               v9 = uextend.i32 v8
;; @003b                               v10 = bor v6, v9
;; @003b                               brif v10, block3, block2
;;
;;                                 block2:
;; @003b                               v52 = load.i64 notrap aligned readonly can_move v0+8
;; @003b                               v12 = load.i64 notrap aligned readonly can_move v52+32
;; @003b                               v11 = uextend.i64 v2
;; @003b                               v13 = iadd v12, v11
;; @003b                               v14 = iconst.i64 8
;; @003b                               v15 = iadd v13, v14  ; v14 = 8
;; @003b                               v16 = load.i64 user2 region0 v15
;;                                     v51 = iconst.i64 1
;; @003b                               v17 = iadd v16, v51  ; v51 = 1
;; @003b                               store user2 region0 v17, v15
;; @003b                               jump block3
;;
;;                                 block3:
;;                                     v68 = iadd.i64 v0, v55  ; v55 = 48
;; @003b                               store.i32 notrap aligned v2, v68
;;                                     v69 = iconst.i32 1
;;                                     v70 = band.i32 v5, v69  ; v69 = 1
;;                                     v71 = iconst.i32 0
;;                                     v72 = icmp.i32 eq v5, v71  ; v71 = 0
;; @003b                               v26 = uextend.i32 v72
;; @003b                               v27 = bor v70, v26
;; @003b                               brif v27, block7, block4
;;
;;                                 block4:
;;                                     v73 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v74 = load.i64 notrap aligned readonly can_move v73+32
;; @003b                               v28 = uextend.i64 v5
;; @003b                               v30 = iadd v74, v28
;;                                     v75 = iconst.i64 8
;; @003b                               v32 = iadd v30, v75  ; v75 = 8
;; @003b                               v33 = load.i64 user2 region0 v32
;;                                     v76 = iconst.i64 1
;;                                     v66 = icmp eq v33, v76  ; v76 = 1
;; @003b                               brif v66, block5, block6
;;
;;                                 block5 cold:
;; @003b                               call fn0(v0, v5)
;; @003b                               jump block7
;;
;;                                 block6:
;;                                     v45 = iconst.i64 -1
;; @003b                               v34 = iadd.i64 v33, v45  ; v45 = -1
;;                                     v77 = iadd.i64 v30, v75  ; v75 = 8
;; @003b                               store user2 region0 v34, v77
;; @003b                               jump block7
;;
;;                                 block7:
;; @003d                               jump block1
;;
;;                                 block1:
;; @003d                               return
;; }
