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
;;     region3 = 40 "VMContext+0x28"
;;     region4 = 268435488 "VMStoreContext+0x20"
;;     region5 = 2147483648 "GcHeap"
;;     region6 = 268435496 "VMStoreContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v138 = stack_addr.i64 ss2
;;                                     store notrap v2, v138
;;                                     v139 = stack_addr.i64 ss1
;;                                     store notrap v3, v139
;;                                     v140 = stack_addr.i64 ss0
;;                                     store notrap v4, v140
;; @0025                               v15 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0025                               v16 = load.i32 notrap aligned v15
;; @0025                               v17 = load.i32 notrap aligned v15+4
;; @0025                               v23 = uextend.i64 v16
;; @0025                               v11 = iconst.i64 32
;; @0025                               v24 = iadd v23, v11  ; v11 = 32
;; @0025                               v25 = uextend.i64 v17
;; @0025                               v26 = icmp ule v24, v25
;; @0025                               brif v26, block2, block3
;;
;;                                 block2:
;;                                     v260 = iconst.i32 32
;;                                     v166 = iadd.i32 v16, v260  ; v260 = 32
;; @0025                               store notrap aligned v166, v15
;;                                     v261 = iconst.i32 -1476394994
;;                                     v262 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v263 = load.i64 notrap aligned readonly can_move region4 v262+32
;; @0025                               v40 = iadd v263, v23
;; @0025                               store notrap aligned v261, v40  ; v261 = -1476394994
;;                                     v264 = load.i64 notrap aligned readonly can_move region3 v0+40
;;                                     v265 = load.i32 notrap aligned readonly can_move v264
;; @0025                               store notrap aligned v265, v40+4
;;                                     v266 = iconst.i64 32
;; @0025                               istore32 notrap aligned v266, v40+8  ; v266 = 32
;; @0025                               jump block4(v16, v40)
;;
;;                                 block3 cold:
;; @0025                               v27 = iconst.i32 -1476394994
;; @0025                               v28 = load.i64 notrap aligned readonly can_move region3 v0+40
;; @0025                               v29 = load.i32 notrap aligned readonly can_move v28
;;                                     v152 = iconst.i32 32
;; @0025                               v30 = iconst.i32 16
;; @0025                               v31 = call fn0(v0, v27, v29, v152, v30), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v27 = -1476394994, v152 = 32, v30 = 16
;; @0025                               v32 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v33 = load.i64 notrap aligned readonly can_move region4 v32+32
;; @0025                               v34 = uextend.i64 v31
;; @0025                               v35 = iadd v33, v34
;; @0025                               jump block4(v31, v35)
;;
;;                                 block4(v44: i32, v45: i64):
;; @0025                               v6 = iconst.i32 3
;; @0025                               v46 = iconst.i64 16
;; @0025                               v47 = iadd v45, v46  ; v46 = 16
;; @0025                               store user2 region5 v6, v47  ; v6 = 3
;; @0025                               trapz v44, user16
;;                                     v267 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v268 = load.i64 notrap aligned readonly can_move region4 v267+32
;; @0025                               v49 = uextend.i64 v44
;; @0025                               v52 = iadd v268, v49
;; @0025                               v54 = iadd v52, v46  ; v46 = 16
;; @0025                               v55 = load.i32 user2 readonly region5 v54
;; @0025                               trapz v55, user17
;; @0025                               v58 = uextend.i64 v55
;;                                     v143 = iconst.i64 2
;;                                     v172 = ishl v58, v143  ; v143 = 2
;;                                     v269 = iconst.i64 32
;;                                     v270 = ushr v172, v269  ; v269 = 32
;; @0025                               trapnz v270, user2
;;                                     v181 = iconst.i32 2
;;                                     v182 = ishl v55, v181  ; v181 = 2
;; @0025                               v7 = iconst.i32 20
;; @0025                               v64 = uadd_overflow_trap v182, v7, user2  ; v7 = 20
;; @0025                               v68 = uadd_overflow_trap v44, v64, user2
;;                                     v137 = load.i32 notrap v138
;; @0025                               v69 = uextend.i64 v68
;; @0025                               v72 = iadd v268, v69
;; @0025                               v73 = isub v64, v7  ; v7 = 20
;; @0025                               v74 = uextend.i64 v73
;; @0025                               v75 = isub v72, v74
;; @0025                               store user2 little region5 v137, v75
;; @0025                               v83 = load.i32 user2 readonly region5 v54
;; @0025                               v76 = iconst.i32 1
;;                                     v199 = icmp ugt v83, v76  ; v76 = 1
;; @0025                               trapz v199, user17
;; @0025                               v86 = uextend.i64 v83
;;                                     v201 = ishl v86, v143  ; v143 = 2
;;                                     v271 = ushr v201, v269  ; v269 = 32
;; @0025                               trapnz v271, user2
;;                                     v208 = ishl v83, v181  ; v181 = 2
;; @0025                               v92 = uadd_overflow_trap v208, v7, user2  ; v7 = 20
;; @0025                               v96 = uadd_overflow_trap v44, v92, user2
;;                                     v135 = load.i32 notrap v139
;; @0025                               v97 = uextend.i64 v96
;; @0025                               v100 = iadd v268, v97
;;                                     v221 = iconst.i32 24
;; @0025                               v101 = isub v92, v221  ; v221 = 24
;; @0025                               v102 = uextend.i64 v101
;; @0025                               v103 = isub v100, v102
;; @0025                               store user2 little region5 v135, v103
;; @0025                               v111 = load.i32 user2 readonly region5 v54
;;                                     v227 = icmp ugt v111, v181  ; v181 = 2
;; @0025                               trapz v227, user17
;; @0025                               v114 = uextend.i64 v111
;;                                     v229 = ishl v114, v143  ; v143 = 2
;;                                     v272 = ushr v229, v269  ; v269 = 32
;; @0025                               trapnz v272, user2
;;                                     v236 = ishl v111, v181  ; v181 = 2
;; @0025                               v120 = uadd_overflow_trap v236, v7, user2  ; v7 = 20
;; @0025                               v124 = uadd_overflow_trap v44, v120, user2
;;                                     v133 = load.i32 notrap v140
;; @0025                               v125 = uextend.i64 v124
;; @0025                               v128 = iadd v268, v125
;;                                     v254 = iconst.i32 28
;; @0025                               v129 = isub v120, v254  ; v254 = 28
;; @0025                               v130 = uextend.i64 v129
;; @0025                               v131 = isub v128, v130
;; @0025                               store user2 little region5 v133, v131
;; @0029                               jump block1(v44)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
