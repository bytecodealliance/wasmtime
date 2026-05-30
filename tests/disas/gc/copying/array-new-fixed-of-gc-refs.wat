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
;; @0025                               v15 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v16 = load.i32 notrap aligned v15
;; @0025                               v17 = load.i32 notrap aligned v15+4
;; @0025                               v23 = uextend.i64 v16
;;                                     v149 = iconst.i64 32
;; @0025                               v24 = iadd v23, v149  ; v149 = 32
;; @0025                               v25 = uextend.i64 v17
;; @0025                               v26 = icmp ule v24, v25
;; @0025                               brif v26, block2, block3
;;
;;                                 block2:
;;                                     v272 = iconst.i32 32
;;                                     v178 = iadd.i32 v16, v272  ; v272 = 32
;; @0025                               store notrap aligned region0 v178, v15
;;                                     v273 = iconst.i32 -1476394994
;;                                     v274 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v275 = load.i64 notrap aligned readonly can_move v274+32
;; @0025                               v40 = iadd v275, v23
;; @0025                               store notrap aligned v273, v40  ; v273 = -1476394994
;;                                     v276 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v277 = load.i32 notrap aligned readonly can_move v276
;; @0025                               store notrap aligned v277, v40+4
;;                                     v278 = iconst.i64 32
;; @0025                               istore32 notrap aligned v278, v40+8  ; v278 = 32
;; @0025                               jump block4(v16, v40)
;;
;;                                 block3 cold:
;; @0025                               v28 = iconst.i32 -1476394994
;; @0025                               v30 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v31 = load.i32 notrap aligned readonly can_move v30
;;                                     v164 = iconst.i32 32
;; @0025                               v32 = iconst.i32 16
;; @0025                               v33 = call fn0(v0, v28, v31, v164, v32), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v28 = -1476394994, v164 = 32, v32 = 16
;; @0025                               v145 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v34 = load.i64 notrap aligned readonly can_move v145+32
;; @0025                               v35 = uextend.i64 v33
;; @0025                               v36 = iadd v34, v35
;; @0025                               jump block4(v33, v36)
;;
;;                                 block4(v45: i32, v46: i64):
;; @0025                               v6 = iconst.i32 3
;; @0025                               v47 = iconst.i64 16
;; @0025                               v48 = iadd v46, v47  ; v47 = 16
;; @0025                               store user2 region1 v6, v48  ; v6 = 3
;; @0025                               trapz v45, user16
;;                                     v279 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v280 = load.i64 notrap aligned readonly can_move v279+32
;; @0025                               v50 = uextend.i64 v45
;; @0025                               v52 = iadd v280, v50
;; @0025                               v54 = iadd v52, v47  ; v47 = 16
;; @0025                               v55 = load.i32 user2 readonly region1 v54
;; @0025                               v49 = iconst.i32 0
;;                                     v181 = icmp ne v55, v49  ; v49 = 0
;; @0025                               trapz v181, user17
;; @0025                               v58 = uextend.i64 v55
;;                                     v155 = iconst.i64 2
;;                                     v184 = ishl v58, v155  ; v155 = 2
;;                                     v281 = iconst.i64 32
;;                                     v282 = ushr v184, v281  ; v281 = 32
;; @0025                               trapnz v282, user2
;;                                     v193 = iconst.i32 2
;;                                     v194 = ishl v55, v193  ; v193 = 2
;; @0025                               v7 = iconst.i32 20
;; @0025                               v63 = uadd_overflow_trap v194, v7, user2  ; v7 = 20
;; @0025                               v67 = uadd_overflow_trap v45, v63, user2
;;                                     v126 = load.i32 notrap v152
;; @0025                               v68 = uextend.i64 v67
;; @0025                               v70 = iadd v280, v68
;; @0025                               v71 = isub v63, v7  ; v7 = 20
;; @0025                               v72 = uextend.i64 v71
;; @0025                               v73 = isub v70, v72
;; @0025                               store user2 little region1 v126, v73
;; @0025                               v80 = load.i32 user2 readonly region1 v54
;; @0025                               v74 = iconst.i32 1
;;                                     v211 = icmp ugt v80, v74  ; v74 = 1
;; @0025                               trapz v211, user17
;; @0025                               v83 = uextend.i64 v80
;;                                     v213 = ishl v83, v155  ; v155 = 2
;;                                     v283 = ushr v213, v281  ; v281 = 32
;; @0025                               trapnz v283, user2
;;                                     v220 = ishl v80, v193  ; v193 = 2
;; @0025                               v88 = uadd_overflow_trap v220, v7, user2  ; v7 = 20
;; @0025                               v92 = uadd_overflow_trap v45, v88, user2
;;                                     v125 = load.i32 notrap v151
;; @0025                               v93 = uextend.i64 v92
;; @0025                               v95 = iadd v280, v93
;;                                     v233 = iconst.i32 24
;; @0025                               v96 = isub v88, v233  ; v233 = 24
;; @0025                               v97 = uextend.i64 v96
;; @0025                               v98 = isub v95, v97
;; @0025                               store user2 little region1 v125, v98
;; @0025                               v105 = load.i32 user2 readonly region1 v54
;;                                     v239 = icmp ugt v105, v193  ; v193 = 2
;; @0025                               trapz v239, user17
;; @0025                               v108 = uextend.i64 v105
;;                                     v241 = ishl v108, v155  ; v155 = 2
;;                                     v284 = ushr v241, v281  ; v281 = 32
;; @0025                               trapnz v284, user2
;;                                     v248 = ishl v105, v193  ; v193 = 2
;; @0025                               v113 = uadd_overflow_trap v248, v7, user2  ; v7 = 20
;; @0025                               v117 = uadd_overflow_trap v45, v113, user2
;;                                     v124 = load.i32 notrap v150
;; @0025                               v118 = uextend.i64 v117
;; @0025                               v120 = iadd v280, v118
;;                                     v266 = iconst.i32 28
;; @0025                               v121 = isub v113, v266  ; v266 = 28
;; @0025                               v122 = uextend.i64 v121
;; @0025                               v123 = isub v120, v122
;; @0025                               store user2 little region1 v124, v123
;; @0029                               jump block1(v45)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
