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
;; @0034                               v3 = iconst.i64 48
;; @0034                               v4 = iadd v0, v3  ; v3 = 48
;; @0034                               v5 = load.i32 notrap aligned region2 v4
;;                                     v80 = stack_addr.i64 ss0
;;                                     store notrap v5, v80
;; @0034                               v6 = iconst.i32 1
;; @0034                               v7 = band v5, v6  ; v6 = 1
;; @0034                               v8 = iconst.i32 0
;; @0034                               v9 = icmp eq v5, v8  ; v8 = 0
;; @0034                               v10 = uextend.i32 v9
;; @0034                               v11 = bor v7, v10
;; @0034                               brif v11, block4, block2
;;
;;                                 block2:
;; @0034                               v13 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0034                               v14 = load.i64 notrap aligned readonly can_move region3 v13+32
;; @0034                               v12 = uextend.i64 v5
;; @0034                               v15 = iadd v14, v12
;; @0034                               v16 = load.i32 user2 region5 v15
;; @0034                               v17 = iconst.i32 2
;; @0034                               v18 = band v16, v17  ; v17 = 2
;; @0034                               brif v18, block4, block3
;;
;;                                 block3:
;; @0034                               v19 = load.i64 notrap aligned readonly can_move region6 v0+32
;; @0034                               v20 = load.i32 user2 region5 v19
;; @0034                               v25 = iconst.i64 16
;; @0034                               v26 = iadd.i64 v15, v25  ; v25 = 16
;; @0034                               store user2 region5 v20, v26
;;                                     v81 = iconst.i32 2
;;                                     v82 = bor.i32 v16, v81  ; v81 = 2
;; @0034                               store user2 region5 v82, v15
;; @0034                               v37 = iconst.i64 8
;; @0034                               v38 = iadd.i64 v15, v37  ; v37 = 8
;; @0034                               v39 = load.i64 user2 region5 v38
;; @0034                               v40 = iconst.i64 1
;; @0034                               v41 = iadd v39, v40  ; v40 = 1
;; @0034                               store user2 region5 v41, v38
;; @0034                               store.i32 user2 region5 v5, v19
;; @0034                               v49 = load.i32 notrap aligned v19+4
;;                                     v83 = iconst.i32 1
;;                                     v84 = iadd v49, v83  ; v83 = 1
;; @0034                               store notrap aligned v84, v19+4
;; @0034                               v56 = load.i32 notrap aligned v19+8
;; @0034                               v57 = iadd v56, v56
;; @0034                               v58 = iconst.i32 1024
;; @0034                               v59 = umax v57, v58  ; v58 = 1024
;; @0034                               v60 = icmp uge v84, v59
;; @0034                               brif v60, block5, block6
;;
;;                                 block5 cold:
;; @0034                               v61 = call fn0(v0), stack_map=[i32 @ ss0+0]
;; @0034                               jump block6
;;
;;                                 block6:
;; @0034                               jump block4
;;
;;                                 block4:
;;                                     v63 = load.i32 notrap v80
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return v63
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
