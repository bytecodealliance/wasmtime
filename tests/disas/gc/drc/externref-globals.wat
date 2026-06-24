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
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 1879048192 "DefinedGlobal(StaticModuleIndex(0), DefinedGlobalIndex(0))"
;;     region3 = 268435488 "VMStoreContext+0x20"
;;     region4 = 268435496 "VMStoreContext+0x28"
;;     region5 = 2147483648 "GcHeap"
;;     region6 = 32 "VMContext+0x20"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:45 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0034                               v2 = iconst.i64 48
;; @0034                               v3 = iadd v0, v2  ; v2 = 48
;; @0034                               v4 = load.i32 notrap aligned region2 v3
;;                                     v79 = stack_addr.i64 ss0
;;                                     store notrap v4, v79
;; @0034                               v5 = iconst.i32 1
;; @0034                               v6 = band v4, v5  ; v5 = 1
;; @0034                               v7 = iconst.i32 0
;; @0034                               v8 = icmp eq v4, v7  ; v7 = 0
;; @0034                               v9 = uextend.i32 v8
;; @0034                               v10 = bor v6, v9
;; @0034                               brif v10, block4, block2
;;
;;                                 block2:
;; @0034                               v12 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0034                               v13 = load.i64 notrap aligned readonly can_move region3 v12+32
;; @0034                               v11 = uextend.i64 v4
;; @0034                               v14 = iadd v13, v11
;; @0034                               v15 = load.i32 user2 region5 v14
;; @0034                               v16 = iconst.i32 2
;; @0034                               v17 = band v15, v16  ; v16 = 2
;; @0034                               brif v17, block4, block3
;;
;;                                 block3:
;; @0034                               v18 = load.i64 notrap aligned readonly can_move region6 v0+32
;; @0034                               v19 = load.i32 user2 region5 v18
;; @0034                               v24 = iconst.i64 16
;; @0034                               v25 = iadd.i64 v14, v24  ; v24 = 16
;; @0034                               store user2 region5 v19, v25
;;                                     v80 = iconst.i32 2
;;                                     v81 = bor.i32 v15, v80  ; v80 = 2
;; @0034                               store user2 region5 v81, v14
;; @0034                               v36 = iconst.i64 8
;; @0034                               v37 = iadd.i64 v14, v36  ; v36 = 8
;; @0034                               v38 = load.i64 user2 region5 v37
;; @0034                               v39 = iconst.i64 1
;; @0034                               v40 = iadd v38, v39  ; v39 = 1
;; @0034                               store user2 region5 v40, v37
;; @0034                               store.i32 user2 region5 v4, v18
;; @0034                               v48 = load.i32 notrap aligned v18+4
;;                                     v82 = iconst.i32 1
;;                                     v83 = iadd v48, v82  ; v82 = 1
;; @0034                               store notrap aligned v83, v18+4
;; @0034                               v55 = load.i32 notrap aligned v18+8
;; @0034                               v56 = iadd v55, v55
;; @0034                               v57 = iconst.i32 1024
;; @0034                               v58 = umax v56, v57  ; v57 = 1024
;; @0034                               v59 = icmp uge v83, v58
;; @0034                               brif v59, block5, block6
;;
;;                                 block5 cold:
;; @0034                               v60 = call fn0(v0), stack_map=[i32 @ ss0+0]
;; @0034                               jump block6
;;
;;                                 block6:
;; @0034                               jump block4
;;
;;                                 block4:
;; @0036                               jump block1
;;
;;                                 block1:
;;                                     v62 = load.i32 notrap v79
;; @0036                               return v62
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 1879048192 "DefinedGlobal(StaticModuleIndex(0), DefinedGlobalIndex(0))"
;;     region3 = 268435488 "VMStoreContext+0x20"
;;     region4 = 268435496 "VMStoreContext+0x28"
;;     region5 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u805306368:22 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003b                               v3 = iconst.i64 48
;; @003b                               v4 = iadd v0, v3  ; v3 = 48
;; @003b                               v5 = load.i32 notrap aligned region2 v4
;; @003b                               v6 = iconst.i32 1
;; @003b                               v7 = band v2, v6  ; v6 = 1
;; @003b                               v8 = iconst.i32 0
;; @003b                               v9 = icmp eq v2, v8  ; v8 = 0
;; @003b                               v10 = uextend.i32 v9
;; @003b                               v11 = bor v7, v10
;; @003b                               brif v11, block3, block2
;;
;;                                 block2:
;; @003b                               v13 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003b                               v14 = load.i64 notrap aligned readonly can_move region3 v13+32
;; @003b                               v12 = uextend.i64 v2
;; @003b                               v15 = iadd v14, v12
;; @003b                               v16 = iconst.i64 8
;; @003b                               v17 = iadd v15, v16  ; v16 = 8
;; @003b                               v18 = load.i64 user2 region5 v17
;; @003b                               v19 = iconst.i64 1
;; @003b                               v20 = iadd v18, v19  ; v19 = 1
;; @003b                               store user2 region5 v20, v17
;; @003b                               jump block3
;;
;;                                 block3:
;;                                     v62 = iadd.i64 v0, v3  ; v3 = 48
;; @003b                               store.i32 notrap aligned region2 v2, v62
;;                                     v63 = iconst.i32 1
;;                                     v64 = band.i32 v5, v63  ; v63 = 1
;;                                     v65 = iconst.i32 0
;;                                     v66 = icmp.i32 eq v5, v65  ; v65 = 0
;; @003b                               v31 = uextend.i32 v66
;; @003b                               v32 = bor v64, v31
;; @003b                               brif v32, block7, block4
;;
;;                                 block4:
;;                                     v67 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v68 = load.i64 notrap aligned readonly can_move region3 v67+32
;; @003b                               v33 = uextend.i64 v5
;; @003b                               v36 = iadd v68, v33
;;                                     v69 = iconst.i64 8
;; @003b                               v38 = iadd v36, v69  ; v69 = 8
;; @003b                               v39 = load.i64 user2 region5 v38
;;                                     v70 = iconst.i64 1
;;                                     v60 = icmp eq v39, v70  ; v70 = 1
;; @003b                               brif v60, block5, block6
;;
;;                                 block5 cold:
;; @003b                               call fn0(v0, v5)
;; @003b                               jump block7
;;
;;                                 block6:
;; @003b                               v40 = iconst.i64 -1
;; @003b                               v41 = iadd.i64 v39, v40  ; v40 = -1
;;                                     v71 = iadd.i64 v36, v69  ; v69 = 8
;; @003b                               store user2 region5 v41, v71
;; @003b                               jump block7
;;
;;                                 block7:
;; @003d                               jump block1
;;
;;                                 block1:
;; @003d                               return
;; }
