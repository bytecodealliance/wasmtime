;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (array (mut i64)))

  (func (param i64 i64 i64) (result (ref $ty))
    (array.new_fixed $ty 3 (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i64, i64) -> i32 tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v14 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0025                               v15 = load.i32 notrap aligned region3 v14
;; @0025                               v16 = load.i32 notrap aligned region4 v14+4
;; @0025                               v22 = uextend.i64 v15
;;                                     v141 = iconst.i64 48
;; @0025                               v23 = iadd v22, v141  ; v141 = 48
;; @0025                               v24 = uextend.i64 v16
;; @0025                               v25 = icmp ule v23, v24
;; @0025                               brif v25, block2, block3
;;
;;                                 block2:
;;                                     v241 = iconst.i32 48
;;                                     v155 = iadd.i32 v15, v241  ; v241 = 48
;; @0025                               store notrap aligned region3 v155, v14
;;                                     v242 = iconst.i32 -1476395002
;;                                     v243 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v244 = load.i64 notrap aligned readonly can_move region7 v243+32
;; @0025                               v39 = iadd v244, v22
;; @0025                               store user2 region8 v242, v39  ; v242 = -1476395002
;;                                     v245 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v246 = load.i32 notrap aligned readonly can_move region6 v245
;; @0025                               store user2 region8 v246, v39+4
;;                                     v247 = iconst.i64 48
;; @0025                               istore32 user2 region8 v247, v39+8  ; v247 = 48
;; @0025                               jump block4(v15, v39)
;;
;;                                 block3 cold:
;; @0025                               v26 = iconst.i32 -1476395002
;; @0025                               v27 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @0025                               v28 = load.i32 notrap aligned readonly can_move region6 v27
;;                                     v140 = iconst.i32 48
;; @0025                               v29 = iconst.i32 16
;; @0025                               v30 = call fn0(v0, v26, v28, v140, v29)  ; v26 = -1476395002, v140 = 48, v29 = 16
;; @0025                               v31 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v32 = load.i64 notrap aligned readonly can_move region7 v31+32
;; @0025                               v33 = uextend.i64 v30
;; @0025                               v34 = iadd v32, v33
;; @0025                               jump block4(v30, v34)
;;
;;                                 block4(v43: i32, v44: i64):
;; @0025                               v5 = iconst.i32 3
;; @0025                               v45 = iconst.i64 16
;; @0025                               v46 = iadd v44, v45  ; v45 = 16
;; @0025                               store user2 region8 v5, v46  ; v5 = 3
;; @0025                               trapz v43, user16
;;                                     v248 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v249 = load.i64 notrap aligned readonly can_move region7 v248+32
;; @0025                               v48 = uextend.i64 v43
;; @0025                               v51 = iadd v249, v48
;; @0025                               v53 = iadd v51, v45  ; v45 = 16
;; @0025                               v54 = load.i32 user2 readonly region8 v53
;; @0025                               trapz v54, user17
;; @0025                               v57 = uextend.i64 v54
;;                                     v131 = iconst.i64 3
;;                                     v162 = ishl v57, v131  ; v131 = 3
;; @0025                               v10 = iconst.i64 32
;; @0025                               v60 = ushr v162, v10  ; v10 = 32
;; @0025                               trapnz v60, user2
;;                                     v169 = ishl v54, v5  ; v5 = 3
;; @0025                               v6 = iconst.i32 24
;; @0025                               v63 = uadd_overflow_trap v169, v6, user2  ; v6 = 24
;; @0025                               v67 = uadd_overflow_trap v43, v63, user2
;; @0025                               v68 = uextend.i64 v67
;; @0025                               v71 = iadd v249, v68
;; @0025                               v72 = isub v63, v6  ; v6 = 24
;; @0025                               v73 = uextend.i64 v72
;; @0025                               v74 = isub v71, v73
;; @0025                               store.i64 user2 little region8 v2, v74
;; @0025                               v82 = load.i32 user2 readonly region8 v53
;; @0025                               v75 = iconst.i32 1
;;                                     v185 = icmp ugt v82, v75  ; v75 = 1
;; @0025                               trapz v185, user17
;; @0025                               v85 = uextend.i64 v82
;;                                     v188 = ishl v85, v131  ; v131 = 3
;; @0025                               v88 = ushr v188, v10  ; v10 = 32
;; @0025                               trapnz v88, user2
;;                                     v193 = ishl v82, v5  ; v5 = 3
;; @0025                               v91 = uadd_overflow_trap v193, v6, user2  ; v6 = 24
;; @0025                               v95 = uadd_overflow_trap v43, v91, user2
;; @0025                               v96 = uextend.i64 v95
;; @0025                               v99 = iadd v249, v96
;;                                     v205 = iconst.i32 32
;; @0025                               v100 = isub v91, v205  ; v205 = 32
;; @0025                               v101 = uextend.i64 v100
;; @0025                               v102 = isub v99, v101
;; @0025                               store.i64 user2 little region8 v3, v102
;; @0025                               v110 = load.i32 user2 readonly region8 v53
;; @0025                               v103 = iconst.i32 2
;;                                     v211 = icmp ugt v110, v103  ; v103 = 2
;; @0025                               trapz v211, user17
;; @0025                               v113 = uextend.i64 v110
;;                                     v214 = ishl v113, v131  ; v131 = 3
;; @0025                               v116 = ushr v214, v10  ; v10 = 32
;; @0025                               trapnz v116, user2
;;                                     v219 = ishl v110, v5  ; v5 = 3
;; @0025                               v119 = uadd_overflow_trap v219, v6, user2  ; v6 = 24
;; @0025                               v123 = uadd_overflow_trap v43, v119, user2
;; @0025                               v124 = uextend.i64 v123
;; @0025                               v127 = iadd v249, v124
;;                                     v235 = iconst.i32 40
;; @0025                               v128 = isub v119, v235  ; v235 = 40
;; @0025                               v129 = uextend.i64 v128
;; @0025                               v130 = isub v127, v129
;; @0025                               store.i64 user2 little region8 v4, v130
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v43
;; }
