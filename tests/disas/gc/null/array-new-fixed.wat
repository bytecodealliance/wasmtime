;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param i64 i64 i64) (result (ref $ty))
    (array.new_fixed $ty 3 (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i64, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 2147483648 "GcHeap"
;;     region4 = 268435496 "VMStoreContext+0x28"
;;     region5 = 268435488 "VMStoreContext+0x20"
;;     region6 = 40 "VMContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v18 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0025                               v19 = load.i32 user2 region3 v18
;;                                     v143 = iconst.i32 7
;; @0025                               v22 = uadd_overflow_trap v19, v143, user18  ; v143 = 7
;;                                     v149 = iconst.i32 -8
;; @0025                               v24 = band v22, v149  ; v149 = -8
;;                                     v136 = iconst.i32 40
;; @0025                               v25 = uadd_overflow_trap v24, v136, user18  ; v136 = 40
;; @0025                               v27 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v28 = load.i64 notrap aligned region4 v27+40
;; @0025                               v26 = uextend.i64 v25
;; @0025                               v29 = icmp ule v26, v28
;; @0025                               brif v29, block2, block3
;;
;;                                 block2:
;;                                     v150 = iconst.i32 -1476394968
;; @0025                               v33 = load.i64 notrap aligned readonly can_move region5 v27+32
;;                                     v245 = band.i32 v22, v149  ; v149 = -8
;;                                     v246 = uextend.i64 v245
;; @0025                               v35 = iadd v33, v246
;; @0025                               store user2 region3 v150, v35  ; v150 = -1476394968
;; @0025                               v38 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @0025                               v39 = load.i32 notrap aligned readonly can_move v38
;; @0025                               store user2 region3 v39, v35+4
;; @0025                               store.i32 user2 region3 v25, v18
;; @0025                               v6 = iconst.i32 3
;; @0025                               v9 = iconst.i64 8
;; @0025                               v41 = iadd v35, v9  ; v9 = 8
;; @0025                               store user2 region3 v6, v41  ; v6 = 3
;; @0025                               trapz v245, user16
;;                                     v247 = iconst.i32 40
;; @0025                               v62 = uadd_overflow_trap v245, v247, user2  ; v247 = 40
;; @0025                               v63 = uextend.i64 v62
;; @0025                               v66 = iadd v33, v63
;;                                     v127 = iconst.i64 24
;; @0025                               v69 = isub v66, v127  ; v127 = 24
;; @0025                               store.i64 user2 little region3 v2, v69
;; @0025                               v77 = load.i32 user2 readonly region3 v41
;; @0025                               v70 = iconst.i32 1
;;                                     v186 = icmp ugt v77, v70  ; v70 = 1
;; @0025                               trapz v186, user17
;; @0025                               v80 = uextend.i64 v77
;;                                     v126 = iconst.i64 3
;;                                     v188 = ishl v80, v126  ; v126 = 3
;; @0025                               v11 = iconst.i64 32
;; @0025                               v83 = ushr v188, v11  ; v11 = 32
;; @0025                               trapnz v83, user2
;;                                     v195 = ishl v77, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 16
;; @0025                               v86 = uadd_overflow_trap v195, v7, user2  ; v7 = 16
;; @0025                               v90 = uadd_overflow_trap v245, v86, user2
;; @0025                               v91 = uextend.i64 v90
;; @0025                               v94 = iadd v33, v91
;;                                     v135 = iconst.i32 24
;; @0025                               v95 = isub v86, v135  ; v135 = 24
;; @0025                               v96 = uextend.i64 v95
;; @0025                               v97 = isub v94, v96
;; @0025                               store.i64 user2 little region3 v3, v97
;; @0025                               v105 = load.i32 user2 readonly region3 v41
;; @0025                               v98 = iconst.i32 2
;;                                     v213 = icmp ugt v105, v98  ; v98 = 2
;; @0025                               trapz v213, user17
;; @0025                               v108 = uextend.i64 v105
;;                                     v215 = ishl v108, v126  ; v126 = 3
;; @0025                               v111 = ushr v215, v11  ; v11 = 32
;; @0025                               trapnz v111, user2
;;                                     v222 = ishl v105, v6  ; v6 = 3
;; @0025                               v114 = uadd_overflow_trap v222, v7, user2  ; v7 = 16
;; @0025                               v118 = uadd_overflow_trap v245, v114, user2
;; @0025                               v119 = uextend.i64 v118
;; @0025                               v122 = iadd v33, v119
;;                                     v239 = iconst.i32 32
;; @0025                               v123 = isub v114, v239  ; v239 = 32
;; @0025                               v124 = uextend.i64 v123
;; @0025                               v125 = isub v122, v124
;; @0025                               store.i64 user2 little region3 v4, v125
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v30 = isub.i64 v26, v28
;; @0025                               v31 = call fn0(v0, v30)
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v248 = band.i32 v22, v149  ; v149 = -8
;; @0029                               return v248
;; }
