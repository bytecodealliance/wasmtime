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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     region2 = 32 "VMContext+0x20"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:45 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0034                               v3 = iconst.i64 48
;; @0034                               v4 = iadd v0, v3  ; v3 = 48
;; @0034                               v5 = load.i32 notrap aligned v4
;;                                     v75 = stack_addr.i64 ss0
;;                                     store notrap v5, v75
;; @0034                               v6 = iconst.i32 1
;; @0034                               v7 = band v5, v6  ; v6 = 1
;; @0034                               v8 = iconst.i32 0
;; @0034                               v9 = icmp eq v5, v8  ; v8 = 0
;; @0034                               v10 = uextend.i32 v9
;; @0034                               v11 = bor v7, v10
;; @0034                               brif v11, block4, block2
;;
;;                                 block2:
;; @0034                               v84 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0034                               v13 = load.i64 notrap aligned readonly can_move v84+32
;; @0034                               v12 = uextend.i64 v5
;; @0034                               v14 = iadd v13, v12
;; @0034                               v15 = load.i32 user2 region1 v14
;; @0034                               v16 = iconst.i32 2
;; @0034                               v17 = band v15, v16  ; v16 = 2
;; @0034                               brif v17, block4, block3
;;
;;                                 block3:
;; @0034                               v18 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0034                               v19 = load.i32 user2 region1 v18
;; @0034                               v23 = iconst.i64 16
;; @0034                               v24 = iadd.i64 v14, v23  ; v23 = 16
;; @0034                               store user2 region1 v19, v24
;;                                     v86 = iconst.i32 2
;;                                     v87 = bor.i32 v15, v86  ; v86 = 2
;; @0034                               store user2 region1 v87, v14
;; @0034                               v33 = iconst.i64 8
;; @0034                               v34 = iadd.i64 v14, v33  ; v33 = 8
;; @0034                               v35 = load.i64 user2 region1 v34
;; @0034                               v36 = iconst.i64 1
;; @0034                               v37 = iadd v35, v36  ; v36 = 1
;; @0034                               store user2 region1 v37, v34
;; @0034                               store.i32 user2 region1 v5, v18
;; @0034                               v44 = load.i32 notrap aligned v18+4
;;                                     v88 = iconst.i32 1
;;                                     v89 = iadd v44, v88  ; v88 = 1
;; @0034                               store notrap aligned v89, v18+4
;; @0034                               v51 = load.i32 notrap aligned v18+8
;; @0034                               v52 = iadd v51, v51
;; @0034                               v53 = iconst.i32 1024
;; @0034                               v54 = umax v52, v53  ; v53 = 1024
;; @0034                               v55 = icmp uge v89, v54
;; @0034                               brif v55, block5, block6
;;
;;                                 block5 cold:
;; @0034                               v56 = call fn0(v0), stack_map=[i32 @ ss0+0]
;; @0034                               jump block6
;;
;;                                 block6:
;; @0034                               jump block4
;;
;;                                 block4:
;;                                     v58 = load.i32 notrap v75
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return v58
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u805306368:22 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003b                               v3 = iconst.i64 48
;; @003b                               v4 = iadd v0, v3  ; v3 = 48
;; @003b                               v5 = load.i32 notrap aligned v4
;; @003b                               v6 = iconst.i32 1
;; @003b                               v7 = band v2, v6  ; v6 = 1
;; @003b                               v8 = iconst.i32 0
;; @003b                               v9 = icmp eq v2, v8  ; v8 = 0
;; @003b                               v10 = uextend.i32 v9
;; @003b                               v11 = bor v7, v10
;; @003b                               brif v11, block3, block2
;;
;;                                 block2:
;; @003b                               v52 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003b                               v13 = load.i64 notrap aligned readonly can_move v52+32
;; @003b                               v12 = uextend.i64 v2
;; @003b                               v14 = iadd v13, v12
;; @003b                               v15 = iconst.i64 8
;; @003b                               v16 = iadd v14, v15  ; v15 = 8
;; @003b                               v17 = load.i64 user2 region1 v16
;; @003b                               v18 = iconst.i64 1
;; @003b                               v19 = iadd v17, v18  ; v18 = 1
;; @003b                               store user2 region1 v19, v16
;; @003b                               jump block3
;;
;;                                 block3:
;;                                     v66 = iadd.i64 v0, v3  ; v3 = 48
;; @003b                               store.i32 notrap aligned v2, v66
;;                                     v67 = iconst.i32 1
;;                                     v68 = band.i32 v5, v67  ; v67 = 1
;;                                     v69 = iconst.i32 0
;;                                     v70 = icmp.i32 eq v5, v69  ; v69 = 0
;; @003b                               v29 = uextend.i32 v70
;; @003b                               v30 = bor v68, v29
;; @003b                               brif v30, block7, block4
;;
;;                                 block4:
;;                                     v71 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v72 = load.i64 notrap aligned readonly can_move v71+32
;; @003b                               v31 = uextend.i64 v5
;; @003b                               v33 = iadd v72, v31
;;                                     v73 = iconst.i64 8
;; @003b                               v35 = iadd v33, v73  ; v73 = 8
;; @003b                               v36 = load.i64 user2 region1 v35
;;                                     v74 = iconst.i64 1
;;                                     v64 = icmp eq v36, v74  ; v74 = 1
;; @003b                               brif v64, block5, block6
;;
;;                                 block5 cold:
;; @003b                               call fn0(v0, v5)
;; @003b                               jump block7
;;
;;                                 block6:
;; @003b                               v37 = iconst.i64 -1
;; @003b                               v38 = iadd.i64 v36, v37  ; v37 = -1
;;                                     v75 = iadd.i64 v33, v73  ; v73 = 8
;; @003b                               store user2 region1 v38, v75
;; @003b                               jump block7
;;
;;                                 block7:
;; @003d                               jump block1
;;
;;                                 block1:
;; @003d                               return
;; }
