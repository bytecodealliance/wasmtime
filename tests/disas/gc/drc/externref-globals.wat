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
;; @0034                               v4 = iconst.i64 48
;; @0034                               v5 = iadd v0, v4  ; v4 = 48
;; @0034                               v6 = load.i32 notrap aligned v5
;;                                     v76 = stack_addr.i64 ss0
;;                                     store notrap v6, v76
;; @0034                               v7 = iconst.i32 1
;; @0034                               v8 = band v6, v7  ; v7 = 1
;; @0034                               v9 = iconst.i32 0
;; @0034                               v10 = icmp eq v6, v9  ; v9 = 0
;; @0034                               v11 = uextend.i32 v10
;; @0034                               v12 = bor v8, v11
;; @0034                               brif v12, block4, block2
;;
;;                                 block2:
;; @0034                               v85 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0034                               v14 = load.i64 notrap aligned readonly can_move v85+32
;; @0034                               v13 = uextend.i64 v6
;; @0034                               v15 = iadd v14, v13
;; @0034                               v16 = load.i32 user2 region1 v15
;; @0034                               v17 = iconst.i32 2
;; @0034                               v18 = band v16, v17  ; v17 = 2
;; @0034                               brif v18, block4, block3
;;
;;                                 block3:
;; @0034                               v19 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0034                               v20 = load.i32 user2 region1 v19
;; @0034                               v24 = iconst.i64 16
;; @0034                               v25 = iadd.i64 v15, v24  ; v24 = 16
;; @0034                               store user2 region1 v20, v25
;;                                     v87 = iconst.i32 2
;;                                     v88 = bor.i32 v16, v87  ; v87 = 2
;; @0034                               store user2 region1 v88, v15
;; @0034                               v34 = iconst.i64 8
;; @0034                               v35 = iadd.i64 v15, v34  ; v34 = 8
;; @0034                               v36 = load.i64 user2 region1 v35
;; @0034                               v37 = iconst.i64 1
;; @0034                               v38 = iadd v36, v37  ; v37 = 1
;; @0034                               store user2 region1 v38, v35
;; @0034                               store.i32 user2 region1 v6, v19
;; @0034                               v45 = load.i32 notrap aligned v19+4
;;                                     v89 = iconst.i32 1
;;                                     v90 = iadd v45, v89  ; v89 = 1
;; @0034                               store notrap aligned v90, v19+4
;; @0034                               v52 = load.i32 notrap aligned v19+8
;; @0034                               v53 = iadd v52, v52
;; @0034                               v54 = iconst.i32 1024
;; @0034                               v55 = umax v53, v54  ; v54 = 1024
;; @0034                               v56 = icmp uge v90, v55
;; @0034                               brif v56, block5, block6
;;
;;                                 block5 cold:
;; @0034                               v57 = call fn0(v0), stack_map=[i32 @ ss0+0]
;; @0034                               jump block6
;;
;;                                 block6:
;; @0034                               jump block4
;;
;;                                 block4:
;;                                     v59 = load.i32 notrap v76
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return v59
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
;; @003b                               v4 = iconst.i64 48
;; @003b                               v5 = iadd v0, v4  ; v4 = 48
;; @003b                               v6 = load.i32 notrap aligned v5
;; @003b                               v7 = iconst.i32 1
;; @003b                               v8 = band v2, v7  ; v7 = 1
;; @003b                               v9 = iconst.i32 0
;; @003b                               v10 = icmp eq v2, v9  ; v9 = 0
;; @003b                               v11 = uextend.i32 v10
;; @003b                               v12 = bor v8, v11
;; @003b                               brif v12, block3, block2
;;
;;                                 block2:
;; @003b                               v53 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003b                               v14 = load.i64 notrap aligned readonly can_move v53+32
;; @003b                               v13 = uextend.i64 v2
;; @003b                               v15 = iadd v14, v13
;; @003b                               v16 = iconst.i64 8
;; @003b                               v17 = iadd v15, v16  ; v16 = 8
;; @003b                               v18 = load.i64 user2 region1 v17
;; @003b                               v19 = iconst.i64 1
;; @003b                               v20 = iadd v18, v19  ; v19 = 1
;; @003b                               store user2 region1 v20, v17
;; @003b                               jump block3
;;
;;                                 block3:
;;                                     v67 = iadd.i64 v0, v4  ; v4 = 48
;; @003b                               store.i32 notrap aligned v2, v67
;;                                     v68 = iconst.i32 1
;;                                     v69 = band.i32 v6, v68  ; v68 = 1
;;                                     v70 = iconst.i32 0
;;                                     v71 = icmp.i32 eq v6, v70  ; v70 = 0
;; @003b                               v30 = uextend.i32 v71
;; @003b                               v31 = bor v69, v30
;; @003b                               brif v31, block7, block4
;;
;;                                 block4:
;;                                     v72 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v73 = load.i64 notrap aligned readonly can_move v72+32
;; @003b                               v32 = uextend.i64 v6
;; @003b                               v34 = iadd v73, v32
;;                                     v74 = iconst.i64 8
;; @003b                               v36 = iadd v34, v74  ; v74 = 8
;; @003b                               v37 = load.i64 user2 region1 v36
;;                                     v75 = iconst.i64 1
;;                                     v65 = icmp eq v37, v75  ; v75 = 1
;; @003b                               brif v65, block5, block6
;;
;;                                 block5 cold:
;; @003b                               call fn0(v0, v6)
;; @003b                               jump block7
;;
;;                                 block6:
;; @003b                               v38 = iconst.i64 -1
;; @003b                               v39 = iadd.i64 v37, v38  ; v38 = -1
;;                                     v76 = iadd.i64 v34, v74  ; v74 = 8
;; @003b                               store user2 region1 v39, v76
;; @003b                               jump block7
;;
;;                                 block7:
;; @003d                               jump block1
;;
;;                                 block1:
;; @003d                               return
;; }
