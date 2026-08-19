;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (array (mut i64)))

  (func (param i64 i32) (result (ref $ty))
    (array.new $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 872415232 "VMCopyingHeapData+0x0"
;;     region4 = 872415236 "VMCopyingHeapData+0x4"
;;     region5 = 40 "VMContext+0x28"
;;     region6 = 1677721600 "TypeIdsArray+0x0"
;;     region7 = 67108896 "VMStoreContext+0x20"
;;     region8 = 536870912 "GcHeap"
;;     region9 = 67108904 "VMStoreContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v5 = uextend.i64 v3
;;                                     v88 = iconst.i64 3
;;                                     v89 = ishl v5, v88  ; v88 = 3
;; @0022                               v8 = iconst.i64 32
;; @0022                               v9 = ushr v89, v8  ; v8 = 32
;; @0022                               trapnz v9, user18
;; @0022                               v4 = iconst.i32 24
;;                                     v95 = iconst.i32 3
;;                                     v96 = ishl v3, v95  ; v95 = 3
;; @0022                               v11 = uadd_overflow_trap v4, v96, user18  ; v4 = 24
;; @0022                               v12 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0022                               v13 = load.i32 notrap aligned region3 v12
;; @0022                               v14 = load.i32 notrap aligned region4 v12+4
;; @0022                               v20 = uextend.i64 v13
;; @0022                               v15 = uextend.i64 v11
;; @0022                               v16 = iconst.i64 15
;; @0022                               v18 = iadd v15, v16  ; v16 = 15
;; @0022                               v17 = iconst.i64 -16
;; @0022                               v19 = band v18, v17  ; v17 = -16
;; @0022                               v21 = iadd v20, v19
;; @0022                               v22 = uextend.i64 v14
;; @0022                               v23 = icmp ule v21, v22
;; @0022                               brif v23, block2, block3
;;
;;                                 block2:
;;                                     v104 = iconst.i32 15
;;                                     v105 = iadd.i32 v11, v104  ; v104 = 15
;;                                     v108 = iconst.i32 -16
;;                                     v109 = band v105, v108  ; v108 = -16
;;                                     v111 = iadd.i32 v13, v109
;; @0022                               store notrap aligned region3 v111, v12
;;                                     v127 = iconst.i32 -1476395002
;;                                     v128 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v129 = load.i64 notrap aligned readonly can_move region7 v128+32
;; @0022                               v37 = iadd v129, v20
;; @0022                               store user2 region8 v127, v37  ; v127 = -1476395002
;;                                     v130 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v131 = load.i32 notrap aligned readonly can_move region6 v130
;; @0022                               store user2 region8 v131, v37+4
;; @0022                               store user2 region8 v109, v37+8
;; @0022                               jump block4(v13, v37)
;;
;;                                 block3 cold:
;; @0022                               v24 = iconst.i32 -1476395002
;; @0022                               v25 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @0022                               v26 = load.i32 notrap aligned readonly can_move region6 v25
;; @0022                               v27 = iconst.i32 16
;; @0022                               v28 = call fn0(v0, v24, v26, v11, v27)  ; v24 = -1476395002, v27 = 16
;; @0022                               v29 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v30 = load.i64 notrap aligned readonly can_move region7 v29+32
;; @0022                               v31 = uextend.i64 v28
;; @0022                               v32 = iadd v30, v31
;; @0022                               jump block4(v28, v32)
;;
;;                                 block4(v42: i32, v43: i64):
;; @0022                               v44 = iconst.i64 16
;; @0022                               v45 = iadd v43, v44  ; v44 = 16
;; @0022                               store.i32 user2 region8 v3, v45
;; @0022                               trapz v42, user16
;;                                     v132 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v133 = load.i64 notrap aligned readonly can_move region7 v132+32
;; @0022                               v47 = uextend.i64 v42
;; @0022                               v50 = iadd v133, v47
;; @0022                               v52 = iadd v50, v44  ; v44 = 16
;; @0022                               v53 = load.i32 user2 readonly region8 v52
;; @0022                               v54 = uextend.i64 v53
;; @0022                               v60 = icmp.i64 ugt v5, v54
;; @0022                               trapnz v60, user17
;; @0022                               v77 = load.i64 notrap aligned region9 v132+40
;; @0022                               v65 = iconst.i64 24
;; @0022                               v66 = iadd v50, v65  ; v65 = 24
;; @0022                               v79 = uadd_overflow_trap v66, v89, user2
;; @0022                               v78 = iadd v133, v77
;; @0022                               v80 = icmp ugt v79, v78
;; @0022                               trapnz v80, user2
;;                                     v113 = iconst.i64 0
;; @0022                               v83 = icmp.i64 eq v5, v113  ; v113 = 0
;; @0022                               v6 = iconst.i64 8
;; @0022                               v81 = iadd v66, v89
;; @0022                               brif v83, block6, block5(v66)
;;
;;                                 block5(v84: i64):
;; @0022                               store.i64 user2 little region8 v2, v84
;;                                     v134 = iconst.i64 8
;;                                     v135 = iadd v84, v134  ; v134 = 8
;; @0022                               v87 = icmp eq v135, v81
;; @0022                               brif v87, block6, block5(v135)
;;
;;                                 block6:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v42
;; }
