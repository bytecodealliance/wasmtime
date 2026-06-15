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
;;     region3 = 3758096384 "VMNullHeapData+0x0"
;;     region4 = 268435496 "VMStoreContext+0x28"
;;     region5 = 268435488 "VMStoreContext+0x20"
;;     region6 = 40 "VMContext+0x28"
;;     region7 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v131 = stack_addr.i64 ss2
;;                                     store notrap v2, v131
;;                                     v132 = stack_addr.i64 ss1
;;                                     store notrap v3, v132
;;                                     v133 = stack_addr.i64 ss0
;;                                     store notrap v4, v133
;; @0025                               v17 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0025                               v18 = load.i32 notrap aligned region3 v17
;;                                     v151 = iconst.i32 7
;; @0025                               v21 = uadd_overflow_trap v18, v151, user18  ; v151 = 7
;;                                     v157 = iconst.i32 -8
;; @0025                               v23 = band v21, v157  ; v157 = -8
;;                                     v144 = iconst.i32 24
;; @0025                               v24 = uadd_overflow_trap v23, v144, user18  ; v144 = 24
;; @0025                               v26 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v27 = load.i64 notrap aligned region4 v26+40
;; @0025                               v25 = uextend.i64 v24
;; @0025                               v28 = icmp ule v25, v27
;; @0025                               brif v28, block2, block3
;;
;;                                 block2:
;;                                     v158 = iconst.i32 -1476394984
;; @0025                               v32 = load.i64 notrap aligned readonly can_move region5 v26+32
;;                                     v256 = band.i32 v21, v157  ; v157 = -8
;;                                     v257 = uextend.i64 v256
;; @0025                               v34 = iadd v32, v257
;; @0025                               store user2 region7 v158, v34  ; v158 = -1476394984
;; @0025                               v37 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @0025                               v38 = load.i32 notrap aligned readonly can_move v37
;; @0025                               store user2 region7 v38, v34+4
;; @0025                               store.i32 notrap aligned region3 v24, v17
;; @0025                               v5 = iconst.i32 3
;; @0025                               v39 = iconst.i64 8
;; @0025                               v40 = iadd v34, v39  ; v39 = 8
;; @0025                               store user2 region7 v5, v40  ; v5 = 3
;; @0025                               trapz v256, user16
;;                                     v258 = iconst.i32 24
;; @0025                               v61 = uadd_overflow_trap v256, v258, user2  ; v258 = 24
;;                                     v130 = load.i32 notrap v131
;; @0025                               v62 = uextend.i64 v61
;; @0025                               v65 = iadd v32, v62
;;                                     v135 = iconst.i64 12
;; @0025                               v68 = isub v65, v135  ; v135 = 12
;; @0025                               store user2 little region7 v130, v68
;; @0025                               v76 = load.i32 user2 readonly region7 v40
;; @0025                               v69 = iconst.i32 1
;;                                     v196 = icmp ugt v76, v69  ; v69 = 1
;; @0025                               trapz v196, user17
;; @0025                               v79 = uextend.i64 v76
;;                                     v136 = iconst.i64 2
;;                                     v198 = ishl v79, v136  ; v136 = 2
;; @0025                               v10 = iconst.i64 32
;; @0025                               v82 = ushr v198, v10  ; v10 = 32
;; @0025                               trapnz v82, user2
;;                                     v175 = iconst.i32 2
;;                                     v205 = ishl v76, v175  ; v175 = 2
;; @0025                               v6 = iconst.i32 12
;; @0025                               v85 = uadd_overflow_trap v205, v6, user2  ; v6 = 12
;; @0025                               v89 = uadd_overflow_trap v256, v85, user2
;;                                     v128 = load.i32 notrap v132
;; @0025                               v90 = uextend.i64 v89
;; @0025                               v93 = iadd v32, v90
;;                                     v218 = iconst.i32 16
;; @0025                               v94 = isub v85, v218  ; v218 = 16
;; @0025                               v95 = uextend.i64 v94
;; @0025                               v96 = isub v93, v95
;; @0025                               store user2 little region7 v128, v96
;; @0025                               v104 = load.i32 user2 readonly region7 v40
;;                                     v224 = icmp ugt v104, v175  ; v175 = 2
;; @0025                               trapz v224, user17
;; @0025                               v107 = uextend.i64 v104
;;                                     v226 = ishl v107, v136  ; v136 = 2
;; @0025                               v110 = ushr v226, v10  ; v10 = 32
;; @0025                               trapnz v110, user2
;;                                     v233 = ishl v104, v175  ; v175 = 2
;; @0025                               v113 = uadd_overflow_trap v233, v6, user2  ; v6 = 12
;; @0025                               v117 = uadd_overflow_trap v256, v113, user2
;;                                     v126 = load.i32 notrap v133
;; @0025                               v118 = uextend.i64 v117
;; @0025                               v121 = iadd v32, v118
;;                                     v250 = iconst.i32 20
;; @0025                               v122 = isub v113, v250  ; v250 = 20
;; @0025                               v123 = uextend.i64 v122
;; @0025                               v124 = isub v121, v123
;; @0025                               store user2 little region7 v126, v124
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v29 = isub.i64 v25, v27
;; @0025                               v30 = call fn0(v0, v29), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v259 = band.i32 v21, v157  ; v157 = -8
;; @0029                               return v259
;; }
