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
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v15 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0025                               v16 = load.i32 notrap aligned v15
;; @0025                               v17 = load.i32 notrap aligned v15+4
;; @0025                               v23 = uextend.i64 v16
;;                                     v142 = iconst.i64 48
;; @0025                               v24 = iadd v23, v142  ; v142 = 48
;; @0025                               v25 = uextend.i64 v17
;; @0025                               v26 = icmp ule v24, v25
;; @0025                               brif v26, block2, block3
;;
;;                                 block2:
;;                                     v248 = iconst.i32 48
;;                                     v156 = iadd.i32 v16, v248  ; v248 = 48
;; @0025                               store notrap aligned v156, v15
;;                                     v249 = iconst.i32 -1476395002
;;                                     v250 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v251 = load.i64 notrap aligned readonly can_move region4 v250+32
;; @0025                               v40 = iadd v251, v23
;; @0025                               store notrap aligned v249, v40  ; v249 = -1476395002
;;                                     v252 = load.i64 notrap aligned readonly can_move region3 v0+40
;;                                     v253 = load.i32 notrap aligned readonly can_move v252
;; @0025                               store notrap aligned v253, v40+4
;;                                     v254 = iconst.i64 48
;; @0025                               istore32 notrap aligned v254, v40+8  ; v254 = 48
;; @0025                               jump block4(v16, v40)
;;
;;                                 block3 cold:
;; @0025                               v27 = iconst.i32 -1476395002
;; @0025                               v28 = load.i64 notrap aligned readonly can_move region3 v0+40
;; @0025                               v29 = load.i32 notrap aligned readonly can_move v28
;;                                     v141 = iconst.i32 48
;; @0025                               v30 = iconst.i32 16
;; @0025                               v31 = call fn0(v0, v27, v29, v141, v30)  ; v27 = -1476395002, v141 = 48, v30 = 16
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
;;                                     v255 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v256 = load.i64 notrap aligned readonly can_move region4 v255+32
;; @0025                               v49 = uextend.i64 v44
;; @0025                               v52 = iadd v256, v49
;; @0025                               v54 = iadd v52, v46  ; v46 = 16
;; @0025                               v55 = load.i32 user2 readonly region5 v54
;; @0025                               trapz v55, user17
;; @0025                               v58 = uextend.i64 v55
;;                                     v132 = iconst.i64 3
;;                                     v162 = ishl v58, v132  ; v132 = 3
;; @0025                               v11 = iconst.i64 32
;; @0025                               v61 = ushr v162, v11  ; v11 = 32
;; @0025                               trapnz v61, user2
;;                                     v171 = ishl v55, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 24
;; @0025                               v64 = uadd_overflow_trap v171, v7, user2  ; v7 = 24
;; @0025                               v68 = uadd_overflow_trap v44, v64, user2
;; @0025                               v69 = uextend.i64 v68
;; @0025                               v72 = iadd v256, v69
;; @0025                               v73 = isub v64, v7  ; v7 = 24
;; @0025                               v74 = uextend.i64 v73
;; @0025                               v75 = isub v72, v74
;; @0025                               store.i64 user2 little region5 v2, v75
;; @0025                               v83 = load.i32 user2 readonly region5 v54
;; @0025                               v76 = iconst.i32 1
;;                                     v188 = icmp ugt v83, v76  ; v76 = 1
;; @0025                               trapz v188, user17
;; @0025                               v86 = uextend.i64 v83
;;                                     v190 = ishl v86, v132  ; v132 = 3
;; @0025                               v89 = ushr v190, v11  ; v11 = 32
;; @0025                               trapnz v89, user2
;;                                     v197 = ishl v83, v6  ; v6 = 3
;; @0025                               v92 = uadd_overflow_trap v197, v7, user2  ; v7 = 24
;; @0025                               v96 = uadd_overflow_trap v44, v92, user2
;; @0025                               v97 = uextend.i64 v96
;; @0025                               v100 = iadd v256, v97
;;                                     v210 = iconst.i32 32
;; @0025                               v101 = isub v92, v210  ; v210 = 32
;; @0025                               v102 = uextend.i64 v101
;; @0025                               v103 = isub v100, v102
;; @0025                               store.i64 user2 little region5 v3, v103
;; @0025                               v111 = load.i32 user2 readonly region5 v54
;; @0025                               v104 = iconst.i32 2
;;                                     v216 = icmp ugt v111, v104  ; v104 = 2
;; @0025                               trapz v216, user17
;; @0025                               v114 = uextend.i64 v111
;;                                     v218 = ishl v114, v132  ; v132 = 3
;; @0025                               v117 = ushr v218, v11  ; v11 = 32
;; @0025                               trapnz v117, user2
;;                                     v225 = ishl v111, v6  ; v6 = 3
;; @0025                               v120 = uadd_overflow_trap v225, v7, user2  ; v7 = 24
;; @0025                               v124 = uadd_overflow_trap v44, v120, user2
;; @0025                               v125 = uextend.i64 v124
;; @0025                               v128 = iadd v256, v125
;;                                     v242 = iconst.i32 40
;; @0025                               v129 = isub v120, v242  ; v242 = 40
;; @0025                               v130 = uextend.i64 v129
;; @0025                               v131 = isub v128, v130
;; @0025                               store.i64 user2 little region5 v4, v131
;; @0029                               jump block1(v44)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
