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
;;     region0 = 32 "VMContext+0x20"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v152 = stack_addr.i64 ss2
;;                                     store notrap v2, v152
;;                                     v151 = stack_addr.i64 ss1
;;                                     store notrap v3, v151
;;                                     v150 = stack_addr.i64 ss0
;;                                     store notrap v4, v150
;; @0025                               v16 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v17 = load.i32 notrap aligned v16
;; @0025                               v18 = load.i32 notrap aligned v16+4
;; @0025                               v24 = uextend.i64 v17
;; @0025                               v11 = iconst.i64 32
;; @0025                               v25 = iadd v24, v11  ; v11 = 32
;; @0025                               v26 = uextend.i64 v18
;; @0025                               v27 = icmp ule v25, v26
;; @0025                               brif v27, block2, block3
;;
;;                                 block2:
;;                                     v272 = iconst.i32 32
;;                                     v178 = iadd.i32 v17, v272  ; v272 = 32
;; @0025                               store notrap aligned region0 v178, v16
;;                                     v273 = iconst.i32 -1476394994
;;                                     v274 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v275 = load.i64 notrap aligned readonly can_move v274+32
;; @0025                               v41 = iadd v275, v24
;; @0025                               store notrap aligned v273, v41  ; v273 = -1476394994
;;                                     v276 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v277 = load.i32 notrap aligned readonly can_move v276
;; @0025                               store notrap aligned v277, v41+4
;;                                     v278 = iconst.i64 32
;; @0025                               istore32 notrap aligned v278, v41+8  ; v278 = 32
;; @0025                               jump block4(v17, v41)
;;
;;                                 block3 cold:
;; @0025                               v29 = iconst.i32 -1476394994
;; @0025                               v31 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v32 = load.i32 notrap aligned readonly can_move v31
;;                                     v164 = iconst.i32 32
;; @0025                               v33 = iconst.i32 16
;; @0025                               v34 = call fn0(v0, v29, v32, v164, v33), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v29 = -1476394994, v164 = 32, v33 = 16
;; @0025                               v146 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v35 = load.i64 notrap aligned readonly can_move v146+32
;; @0025                               v36 = uextend.i64 v34
;; @0025                               v37 = iadd v35, v36
;; @0025                               jump block4(v34, v37)
;;
;;                                 block4(v46: i32, v47: i64):
;; @0025                               v6 = iconst.i32 3
;; @0025                               v48 = iconst.i64 16
;; @0025                               v49 = iadd v47, v48  ; v48 = 16
;; @0025                               store user2 region1 v6, v49  ; v6 = 3
;; @0025                               trapz v46, user16
;;                                     v279 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v280 = load.i64 notrap aligned readonly can_move v279+32
;; @0025                               v51 = uextend.i64 v46
;; @0025                               v53 = iadd v280, v51
;; @0025                               v55 = iadd v53, v48  ; v48 = 16
;; @0025                               v56 = load.i32 user2 readonly region1 v55
;; @0025                               v50 = iconst.i32 0
;;                                     v181 = icmp ne v56, v50  ; v50 = 0
;; @0025                               trapz v181, user17
;; @0025                               v59 = uextend.i64 v56
;;                                     v155 = iconst.i64 2
;;                                     v184 = ishl v59, v155  ; v155 = 2
;;                                     v281 = iconst.i64 32
;;                                     v282 = ushr v184, v281  ; v281 = 32
;; @0025                               trapnz v282, user2
;;                                     v193 = iconst.i32 2
;;                                     v194 = ishl v56, v193  ; v193 = 2
;; @0025                               v7 = iconst.i32 20
;; @0025                               v65 = uadd_overflow_trap v194, v7, user2  ; v7 = 20
;; @0025                               v69 = uadd_overflow_trap v46, v65, user2
;;                                     v130 = load.i32 notrap v152
;; @0025                               v70 = uextend.i64 v69
;; @0025                               v72 = iadd v280, v70
;; @0025                               v73 = isub v65, v7  ; v7 = 20
;; @0025                               v74 = uextend.i64 v73
;; @0025                               v75 = isub v72, v74
;; @0025                               store user2 little region1 v130, v75
;; @0025                               v82 = load.i32 user2 readonly region1 v55
;; @0025                               v76 = iconst.i32 1
;;                                     v211 = icmp ugt v82, v76  ; v76 = 1
;; @0025                               trapz v211, user17
;; @0025                               v85 = uextend.i64 v82
;;                                     v213 = ishl v85, v155  ; v155 = 2
;;                                     v283 = ushr v213, v281  ; v281 = 32
;; @0025                               trapnz v283, user2
;;                                     v220 = ishl v82, v193  ; v193 = 2
;; @0025                               v91 = uadd_overflow_trap v220, v7, user2  ; v7 = 20
;; @0025                               v95 = uadd_overflow_trap v46, v91, user2
;;                                     v129 = load.i32 notrap v151
;; @0025                               v96 = uextend.i64 v95
;; @0025                               v98 = iadd v280, v96
;;                                     v233 = iconst.i32 24
;; @0025                               v99 = isub v91, v233  ; v233 = 24
;; @0025                               v100 = uextend.i64 v99
;; @0025                               v101 = isub v98, v100
;; @0025                               store user2 little region1 v129, v101
;; @0025                               v108 = load.i32 user2 readonly region1 v55
;;                                     v239 = icmp ugt v108, v193  ; v193 = 2
;; @0025                               trapz v239, user17
;; @0025                               v111 = uextend.i64 v108
;;                                     v241 = ishl v111, v155  ; v155 = 2
;;                                     v284 = ushr v241, v281  ; v281 = 32
;; @0025                               trapnz v284, user2
;;                                     v248 = ishl v108, v193  ; v193 = 2
;; @0025                               v117 = uadd_overflow_trap v248, v7, user2  ; v7 = 20
;; @0025                               v121 = uadd_overflow_trap v46, v117, user2
;;                                     v128 = load.i32 notrap v150
;; @0025                               v122 = uextend.i64 v121
;; @0025                               v124 = iadd v280, v122
;;                                     v266 = iconst.i32 28
;; @0025                               v125 = isub v117, v266  ; v266 = 28
;; @0025                               v126 = uextend.i64 v125
;; @0025                               v127 = isub v124, v126
;; @0025                               store user2 little region1 v128, v127
;; @0029                               jump block1(v46)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
