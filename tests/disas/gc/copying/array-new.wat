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
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 3489660928 "VMCopyingHeapData+0x0"
;;     region4 = 3489660932 "VMCopyingHeapData+0x4"
;;     region5 = 40 "VMContext+0x28"
;;     region6 = 268435488 "VMStoreContext+0x20"
;;     region7 = 2147483648 "GcHeap"
;;     region8 = 268435496 "VMStoreContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v5 = uextend.i64 v3
;;                                     v87 = iconst.i64 3
;;                                     v88 = ishl v5, v87  ; v87 = 3
;; @0022                               v8 = iconst.i64 32
;; @0022                               v9 = ushr v88, v8  ; v8 = 32
;; @0022                               trapnz v9, user18
;; @0022                               v4 = iconst.i32 24
;;                                     v94 = iconst.i32 3
;;                                     v95 = ishl v3, v94  ; v94 = 3
;; @0022                               v11 = uadd_overflow_trap v4, v95, user18  ; v4 = 24
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
;;                                     v103 = iconst.i32 15
;;                                     v104 = iadd.i32 v11, v103  ; v103 = 15
;;                                     v107 = iconst.i32 -16
;;                                     v108 = band v104, v107  ; v107 = -16
;;                                     v110 = iadd.i32 v13, v108
;; @0022                               store notrap aligned region3 v110, v12
;;                                     v126 = iconst.i32 -1476395002
;;                                     v127 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v128 = load.i64 notrap aligned readonly can_move region6 v127+32
;; @0022                               v37 = iadd v128, v20
;; @0022                               store user2 region7 v126, v37  ; v126 = -1476395002
;;                                     v129 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v130 = load.i32 notrap aligned readonly can_move v129
;; @0022                               store user2 region7 v130, v37+4
;;                                     v131 = band.i64 v18, v17  ; v17 = -16
;; @0022                               istore32 user2 region7 v131, v37+8
;; @0022                               jump block4(v13, v37)
;;
;;                                 block3 cold:
;; @0022                               v24 = iconst.i32 -1476395002
;; @0022                               v25 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @0022                               v26 = load.i32 notrap aligned readonly can_move v25
;; @0022                               v27 = iconst.i32 16
;; @0022                               v28 = call fn0(v0, v24, v26, v11, v27)  ; v24 = -1476395002, v27 = 16
;; @0022                               v29 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v30 = load.i64 notrap aligned readonly can_move region6 v29+32
;; @0022                               v31 = uextend.i64 v28
;; @0022                               v32 = iadd v30, v31
;; @0022                               jump block4(v28, v32)
;;
;;                                 block4(v41: i32, v42: i64):
;; @0022                               v43 = iconst.i64 16
;; @0022                               v44 = iadd v42, v43  ; v43 = 16
;; @0022                               store.i32 user2 region7 v3, v44
;; @0022                               trapz v41, user16
;;                                     v132 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v133 = load.i64 notrap aligned readonly can_move region6 v132+32
;; @0022                               v46 = uextend.i64 v41
;; @0022                               v49 = iadd v133, v46
;; @0022                               v51 = iadd v49, v43  ; v43 = 16
;; @0022                               v52 = load.i32 user2 readonly region7 v51
;; @0022                               v53 = uextend.i64 v52
;; @0022                               v59 = icmp.i64 ugt v5, v53
;; @0022                               trapnz v59, user17
;; @0022                               v76 = load.i64 notrap aligned region8 v132+40
;; @0022                               v64 = iconst.i64 24
;; @0022                               v65 = iadd v49, v64  ; v64 = 24
;; @0022                               v78 = uadd_overflow_trap v65, v88, user2
;; @0022                               v77 = iadd v133, v76
;; @0022                               v79 = icmp ugt v78, v77
;; @0022                               trapnz v79, user2
;;                                     v112 = iconst.i64 0
;; @0022                               v82 = icmp.i64 eq v5, v112  ; v112 = 0
;; @0022                               v6 = iconst.i64 8
;; @0022                               v80 = iadd v65, v88
;; @0022                               brif v82, block6, block5(v65)
;;
;;                                 block5(v83: i64):
;; @0022                               store.i64 user2 little region7 v2, v83
;;                                     v134 = iconst.i64 8
;;                                     v135 = iadd v83, v134  ; v134 = 8
;; @0022                               v86 = icmp eq v135, v80
;; @0022                               brif v86, block6, block5(v135)
;;
;;                                 block6:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v41
;; }
