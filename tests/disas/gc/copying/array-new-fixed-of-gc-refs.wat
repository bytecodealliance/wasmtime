;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v137 = stack_addr.i64 ss2
;;                                     store notrap v2, v137
;;                                     v138 = stack_addr.i64 ss1
;;                                     store notrap v3, v138
;;                                     v139 = stack_addr.i64 ss0
;;                                     store notrap v4, v139
;; @0025                               v14 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0025                               v15 = load.i32 notrap aligned region3 v14
;; @0025                               v16 = load.i32 notrap aligned region4 v14+4
;; @0025                               v22 = uextend.i64 v15
;; @0025                               v10 = iconst.i64 32
;; @0025                               v23 = iadd v22, v10  ; v10 = 32
;; @0025                               v24 = uextend.i64 v16
;; @0025                               v25 = icmp ule v23, v24
;; @0025                               brif v25, block2, block3
;;
;;                                 block2:
;;                                     v259 = iconst.i32 32
;;                                     v165 = iadd.i32 v15, v259  ; v259 = 32
;; @0025                               store notrap aligned region3 v165, v14
;;                                     v260 = iconst.i32 -1476394994
;;                                     v261 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v262 = load.i64 notrap aligned readonly can_move region6 v261+32
;; @0025                               v39 = iadd v262, v22
;; @0025                               store user2 region7 v260, v39  ; v260 = -1476394994
;;                                     v263 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v264 = load.i32 notrap aligned readonly can_move v263
;; @0025                               store user2 region7 v264, v39+4
;;                                     v265 = iconst.i64 32
;; @0025                               istore32 user2 region7 v265, v39+8  ; v265 = 32
;; @0025                               jump block4(v15, v39)
;;
;;                                 block3 cold:
;; @0025                               v26 = iconst.i32 -1476394994
;; @0025                               v27 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @0025                               v28 = load.i32 notrap aligned readonly can_move v27
;;                                     v151 = iconst.i32 32
;; @0025                               v29 = iconst.i32 16
;; @0025                               v30 = call fn0(v0, v26, v28, v151, v29), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v26 = -1476394994, v151 = 32, v29 = 16
;; @0025                               v31 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v32 = load.i64 notrap aligned readonly can_move region6 v31+32
;; @0025                               v33 = uextend.i64 v30
;; @0025                               v34 = iadd v32, v33
;; @0025                               jump block4(v30, v34)
;;
;;                                 block4(v43: i32, v44: i64):
;; @0025                               v5 = iconst.i32 3
;; @0025                               v45 = iconst.i64 16
;; @0025                               v46 = iadd v44, v45  ; v45 = 16
;; @0025                               store user2 region7 v5, v46  ; v5 = 3
;; @0025                               trapz v43, user16
;;                                     v266 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v267 = load.i64 notrap aligned readonly can_move region6 v266+32
;; @0025                               v48 = uextend.i64 v43
;; @0025                               v51 = iadd v267, v48
;; @0025                               v53 = iadd v51, v45  ; v45 = 16
;; @0025                               v54 = load.i32 user2 readonly region7 v53
;; @0025                               trapz v54, user17
;; @0025                               v57 = uextend.i64 v54
;;                                     v142 = iconst.i64 2
;;                                     v171 = ishl v57, v142  ; v142 = 2
;;                                     v268 = iconst.i64 32
;;                                     v269 = ushr v171, v268  ; v268 = 32
;; @0025                               trapnz v269, user2
;;                                     v180 = iconst.i32 2
;;                                     v181 = ishl v54, v180  ; v180 = 2
;; @0025                               v6 = iconst.i32 20
;; @0025                               v63 = uadd_overflow_trap v181, v6, user2  ; v6 = 20
;; @0025                               v67 = uadd_overflow_trap v43, v63, user2
;;                                     v136 = load.i32 notrap v137
;; @0025                               v68 = uextend.i64 v67
;; @0025                               v71 = iadd v267, v68
;; @0025                               v72 = isub v63, v6  ; v6 = 20
;; @0025                               v73 = uextend.i64 v72
;; @0025                               v74 = isub v71, v73
;; @0025                               store user2 little region7 v136, v74
;; @0025                               v82 = load.i32 user2 readonly region7 v53
;; @0025                               v75 = iconst.i32 1
;;                                     v198 = icmp ugt v82, v75  ; v75 = 1
;; @0025                               trapz v198, user17
;; @0025                               v85 = uextend.i64 v82
;;                                     v200 = ishl v85, v142  ; v142 = 2
;;                                     v270 = ushr v200, v268  ; v268 = 32
;; @0025                               trapnz v270, user2
;;                                     v207 = ishl v82, v180  ; v180 = 2
;; @0025                               v91 = uadd_overflow_trap v207, v6, user2  ; v6 = 20
;; @0025                               v95 = uadd_overflow_trap v43, v91, user2
;;                                     v134 = load.i32 notrap v138
;; @0025                               v96 = uextend.i64 v95
;; @0025                               v99 = iadd v267, v96
;;                                     v220 = iconst.i32 24
;; @0025                               v100 = isub v91, v220  ; v220 = 24
;; @0025                               v101 = uextend.i64 v100
;; @0025                               v102 = isub v99, v101
;; @0025                               store user2 little region7 v134, v102
;; @0025                               v110 = load.i32 user2 readonly region7 v53
;;                                     v226 = icmp ugt v110, v180  ; v180 = 2
;; @0025                               trapz v226, user17
;; @0025                               v113 = uextend.i64 v110
;;                                     v228 = ishl v113, v142  ; v142 = 2
;;                                     v271 = ushr v228, v268  ; v268 = 32
;; @0025                               trapnz v271, user2
;;                                     v235 = ishl v110, v180  ; v180 = 2
;; @0025                               v119 = uadd_overflow_trap v235, v6, user2  ; v6 = 20
;; @0025                               v123 = uadd_overflow_trap v43, v119, user2
;;                                     v132 = load.i32 notrap v139
;; @0025                               v124 = uextend.i64 v123
;; @0025                               v127 = iadd v267, v124
;;                                     v253 = iconst.i32 28
;; @0025                               v128 = isub v119, v253  ; v253 = 28
;; @0025                               v129 = uextend.i64 v128
;; @0025                               v130 = isub v127, v129
;; @0025                               store user2 little region7 v132, v130
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v43
;; }
