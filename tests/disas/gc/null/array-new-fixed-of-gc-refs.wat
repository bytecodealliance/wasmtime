;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (array (mut anyref)))

  (func (param anyref anyref anyref) (result (ref $ty))
    (array.new_fixed $ty 3 (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     ss1 = explicit_slot 4, align = 4
;;     ss2 = explicit_slot 4, align = 4
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v132 = stack_addr.i64 ss2
;;                                     store notrap v2, v132
;;                                     v133 = stack_addr.i64 ss1
;;                                     store notrap v3, v133
;;                                     v134 = stack_addr.i64 ss0
;;                                     store notrap v4, v134
;; @0025                               v18 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0025                               v19 = load.i32 user2 region3 v18
;;                                     v152 = iconst.i32 7
;; @0025                               v22 = uadd_overflow_trap v19, v152, user18  ; v152 = 7
;;                                     v158 = iconst.i32 -8
;; @0025                               v24 = band v22, v158  ; v158 = -8
;;                                     v145 = iconst.i32 24
;; @0025                               v25 = uadd_overflow_trap v24, v145, user18  ; v145 = 24
;; @0025                               v27 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v28 = load.i64 notrap aligned region4 v27+40
;; @0025                               v26 = uextend.i64 v25
;; @0025                               v29 = icmp ule v26, v28
;; @0025                               brif v29, block2, block3
;;
;;                                 block2:
;;                                     v159 = iconst.i32 -1476394984
;; @0025                               v33 = load.i64 notrap aligned readonly can_move region5 v27+32
;;                                     v257 = band.i32 v22, v158  ; v158 = -8
;;                                     v258 = uextend.i64 v257
;; @0025                               v35 = iadd v33, v258
;; @0025                               store user2 region3 v159, v35  ; v159 = -1476394984
;; @0025                               v38 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @0025                               v39 = load.i32 notrap aligned readonly can_move v38
;; @0025                               store user2 region3 v39, v35+4
;; @0025                               store.i32 user2 region3 v25, v18
;; @0025                               v6 = iconst.i32 3
;; @0025                               v40 = iconst.i64 8
;; @0025                               v41 = iadd v35, v40  ; v40 = 8
;; @0025                               store user2 region3 v6, v41  ; v6 = 3
;; @0025                               trapz v257, user16
;;                                     v259 = iconst.i32 24
;; @0025                               v62 = uadd_overflow_trap v257, v259, user2  ; v259 = 24
;;                                     v131 = load.i32 notrap v132
;; @0025                               v63 = uextend.i64 v62
;; @0025                               v66 = iadd v33, v63
;;                                     v136 = iconst.i64 12
;; @0025                               v69 = isub v66, v136  ; v136 = 12
;; @0025                               store user2 little region3 v131, v69
;; @0025                               v77 = load.i32 user2 readonly region3 v41
;; @0025                               v70 = iconst.i32 1
;;                                     v197 = icmp ugt v77, v70  ; v70 = 1
;; @0025                               trapz v197, user17
;; @0025                               v80 = uextend.i64 v77
;;                                     v137 = iconst.i64 2
;;                                     v199 = ishl v80, v137  ; v137 = 2
;; @0025                               v11 = iconst.i64 32
;; @0025                               v83 = ushr v199, v11  ; v11 = 32
;; @0025                               trapnz v83, user2
;;                                     v176 = iconst.i32 2
;;                                     v206 = ishl v77, v176  ; v176 = 2
;; @0025                               v7 = iconst.i32 12
;; @0025                               v86 = uadd_overflow_trap v206, v7, user2  ; v7 = 12
;; @0025                               v90 = uadd_overflow_trap v257, v86, user2
;;                                     v129 = load.i32 notrap v133
;; @0025                               v91 = uextend.i64 v90
;; @0025                               v94 = iadd v33, v91
;;                                     v219 = iconst.i32 16
;; @0025                               v95 = isub v86, v219  ; v219 = 16
;; @0025                               v96 = uextend.i64 v95
;; @0025                               v97 = isub v94, v96
;; @0025                               store user2 little region3 v129, v97
;; @0025                               v105 = load.i32 user2 readonly region3 v41
;;                                     v225 = icmp ugt v105, v176  ; v176 = 2
;; @0025                               trapz v225, user17
;; @0025                               v108 = uextend.i64 v105
;;                                     v227 = ishl v108, v137  ; v137 = 2
;; @0025                               v111 = ushr v227, v11  ; v11 = 32
;; @0025                               trapnz v111, user2
;;                                     v234 = ishl v105, v176  ; v176 = 2
;; @0025                               v114 = uadd_overflow_trap v234, v7, user2  ; v7 = 12
;; @0025                               v118 = uadd_overflow_trap v257, v114, user2
;;                                     v127 = load.i32 notrap v134
;; @0025                               v119 = uextend.i64 v118
;; @0025                               v122 = iadd v33, v119
;;                                     v251 = iconst.i32 20
;; @0025                               v123 = isub v114, v251  ; v251 = 20
;; @0025                               v124 = uextend.i64 v123
;; @0025                               v125 = isub v122, v124
;; @0025                               store user2 little region3 v127, v125
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v30 = isub.i64 v26, v28
;; @0025                               v31 = call fn0(v0, v30), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v260 = band.i32 v22, v158  ; v158 = -8
;; @0029                               return v260
;; }
