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
;; @0034                               v4 = iconst.i64 48
;; @0034                               v5 = iadd v0, v4  ; v4 = 48
;; @0034                               v6 = load.i32 notrap aligned v5
;;                                     v92 = stack_addr.i64 ss0
;;                                     store notrap v6, v92
;;                                     v90 = iconst.i32 1
;; @0034                               v7 = band v6, v90  ; v90 = 1
;; @0034                               v8 = iconst.i32 0
;; @0034                               v9 = icmp eq v6, v8  ; v8 = 0
;; @0034                               v10 = uextend.i32 v9
;; @0034                               v11 = bor v7, v10
;; @0034                               brif v11, block4, block2
;;
;;                                 block2:
;; @0034                               v86 = load.i64 notrap aligned readonly can_move v0+8
;; @0034                               v13 = load.i64 notrap aligned readonly can_move v86+32
;; @0034                               v12 = uextend.i64 v6
;; @0034                               v14 = iadd v13, v12
;; @0034                               v15 = load.i32 user2 region0 v14
;; @0034                               v16 = iconst.i32 2
;; @0034                               v17 = band v15, v16  ; v16 = 2
;; @0034                               brif v17, block4, block3
;;
;;                                 block3:
;; @0034                               v19 = load.i64 notrap aligned readonly can_move region1 v0+32
;; @0034                               v20 = load.i32 user2 region0 v19
;; @0034                               v24 = iconst.i64 16
;; @0034                               v25 = iadd.i64 v14, v24  ; v24 = 16
;; @0034                               store user2 region0 v20, v25
;;                                     v93 = iconst.i32 2
;;                                     v94 = bor.i32 v15, v93  ; v93 = 2
;; @0034                               store user2 region0 v94, v14
;; @0034                               v34 = iconst.i64 8
;; @0034                               v35 = iadd.i64 v14, v34  ; v34 = 8
;; @0034                               v36 = load.i64 user2 region0 v35
;; @0034                               v37 = iconst.i64 1
;; @0034                               v38 = iadd v36, v37  ; v37 = 1
;; @0034                               store user2 region0 v38, v35
;; @0034                               store.i32 user2 region0 v6, v19
;; @0034                               v46 = load.i32 notrap aligned v19+4
;;                                     v95 = iconst.i32 1
;;                                     v96 = iadd v46, v95  ; v95 = 1
;; @0034                               store notrap aligned v96, v19+4
;; @0034                               v56 = load.i32 notrap aligned v19+8
;; @0034                               v57 = iadd v56, v56
;; @0034                               v58 = iconst.i32 1024
;; @0034                               v59 = umax v57, v58  ; v58 = 1024
;; @0034                               v60 = icmp uge v96, v59
;; @0034                               brif v60, block5, block6
;;
;;                                 block5 cold:
;; @0034                               v62 = call fn0(v0), stack_map=[i32 @ ss0+0]
;; @0034                               jump block6
;;
;;                                 block6:
;; @0034                               jump block4
;;
;;                                 block4:
;;                                     v63 = load.i32 notrap v92
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return v63
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
;; @003b                               v4 = iconst.i64 48
;; @003b                               v5 = iadd v0, v4  ; v4 = 48
;; @003b                               v6 = load.i32 notrap aligned v5
;;                                     v55 = iconst.i32 1
;; @003b                               v7 = band v2, v55  ; v55 = 1
;; @003b                               v8 = iconst.i32 0
;; @003b                               v9 = icmp eq v2, v8  ; v8 = 0
;; @003b                               v10 = uextend.i32 v9
;; @003b                               v11 = bor v7, v10
;; @003b                               brif v11, block3, block2
;;
;;                                 block2:
;; @003b                               v53 = load.i64 notrap aligned readonly can_move v0+8
;; @003b                               v13 = load.i64 notrap aligned readonly can_move v53+32
;; @003b                               v12 = uextend.i64 v2
;; @003b                               v14 = iadd v13, v12
;; @003b                               v15 = iconst.i64 8
;; @003b                               v16 = iadd v14, v15  ; v15 = 8
;; @003b                               v17 = load.i64 user2 region0 v16
;; @003b                               v18 = iconst.i64 1
;; @003b                               v19 = iadd v17, v18  ; v18 = 1
;; @003b                               store user2 region0 v19, v16
;; @003b                               jump block3
;;
;;                                 block3:
;;                                     v68 = iadd.i64 v0, v4  ; v4 = 48
;; @003b                               store.i32 notrap aligned v2, v68
;;                                     v69 = iconst.i32 1
;;                                     v70 = band.i32 v6, v69  ; v69 = 1
;;                                     v71 = iconst.i32 0
;;                                     v72 = icmp.i32 eq v6, v71  ; v71 = 0
;; @003b                               v28 = uextend.i32 v72
;; @003b                               v29 = bor v70, v28
;; @003b                               brif v29, block7, block4
;;
;;                                 block4:
;;                                     v73 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v74 = load.i64 notrap aligned readonly can_move v73+32
;; @003b                               v30 = uextend.i64 v6
;; @003b                               v32 = iadd v74, v30
;;                                     v75 = iconst.i64 8
;; @003b                               v34 = iadd v32, v75  ; v75 = 8
;; @003b                               v35 = load.i64 user2 region0 v34
;;                                     v76 = iconst.i64 1
;;                                     v66 = icmp eq v35, v76  ; v76 = 1
;; @003b                               brif v66, block5, block6
;;
;;                                 block5 cold:
;; @003b                               call fn0(v0, v6)
;; @003b                               jump block7
;;
;;                                 block6:
;; @003b                               v36 = iconst.i64 -1
;; @003b                               v37 = iadd.i64 v35, v36  ; v36 = -1
;;                                     v77 = iadd.i64 v32, v75  ; v75 = 8
;; @003b                               store user2 region0 v37, v77
;; @003b                               jump block7
;;
;;                                 block7:
;; @003d                               jump block1
;;
;;                                 block1:
;; @003d                               return
;; }
