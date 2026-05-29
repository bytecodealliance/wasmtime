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
;; @0025                               v14 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v15 = load.i32 notrap aligned v14
;; @0025                               v16 = load.i32 notrap aligned v14+4
;; @0025                               v22 = uextend.i64 v15
;;                                     v148 = iconst.i64 32
;; @0025                               v23 = iadd v22, v148  ; v148 = 32
;; @0025                               v24 = uextend.i64 v16
;; @0025                               v25 = icmp ule v23, v24
;; @0025                               brif v25, block2, block3
;;
;;                                 block2:
;;                                     v272 = iconst.i32 32
;;                                     v178 = iadd.i32 v15, v272  ; v272 = 32
;; @0025                               store notrap aligned region0 v178, v14
;;                                     v273 = iconst.i32 -1476394994
;;                                     v274 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v275 = load.i64 notrap aligned readonly can_move v274+32
;; @0025                               v39 = iadd v275, v22
;; @0025                               store notrap aligned v273, v39  ; v273 = -1476394994
;;                                     v276 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v277 = load.i32 notrap aligned readonly can_move v276
;; @0025                               store notrap aligned v277, v39+4
;;                                     v278 = iconst.i64 32
;; @0025                               istore32 notrap aligned v278, v39+8  ; v278 = 32
;; @0025                               jump block4(v15, v39)
;;
;;                                 block3 cold:
;; @0025                               v27 = iconst.i32 -1476394994
;; @0025                               v29 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v30 = load.i32 notrap aligned readonly can_move v29
;;                                     v164 = iconst.i32 32
;; @0025                               v31 = iconst.i32 16
;; @0025                               v32 = call fn0(v0, v27, v30, v164, v31), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v27 = -1476394994, v164 = 32, v31 = 16
;; @0025                               v144 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v33 = load.i64 notrap aligned readonly can_move v144+32
;; @0025                               v34 = uextend.i64 v32
;; @0025                               v35 = iadd v33, v34
;; @0025                               jump block4(v32, v35)
;;
;;                                 block4(v44: i32, v45: i64):
;; @0025                               v6 = iconst.i32 3
;; @0025                               v46 = iconst.i64 16
;; @0025                               v47 = iadd v45, v46  ; v46 = 16
;; @0025                               store user2 region1 v6, v47  ; v6 = 3
;; @0025                               trapz v44, user16
;;                                     v279 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v280 = load.i64 notrap aligned readonly can_move v279+32
;; @0025                               v49 = uextend.i64 v44
;; @0025                               v51 = iadd v280, v49
;; @0025                               v53 = iadd v51, v46  ; v46 = 16
;; @0025                               v54 = load.i32 user2 readonly region1 v53
;; @0025                               v48 = iconst.i32 0
;;                                     v181 = icmp ne v54, v48  ; v48 = 0
;; @0025                               trapz v181, user17
;; @0025                               v57 = uextend.i64 v54
;;                                     v155 = iconst.i64 2
;;                                     v184 = ishl v57, v155  ; v155 = 2
;;                                     v281 = iconst.i64 32
;;                                     v282 = ushr v184, v281  ; v281 = 32
;; @0025                               trapnz v282, user2
;;                                     v193 = iconst.i32 2
;;                                     v194 = ishl v54, v193  ; v193 = 2
;; @0025                               v7 = iconst.i32 20
;; @0025                               v62 = uadd_overflow_trap v194, v7, user2  ; v7 = 20
;; @0025                               v66 = uadd_overflow_trap v44, v62, user2
;;                                     v125 = load.i32 notrap v152
;; @0025                               v67 = uextend.i64 v66
;; @0025                               v69 = iadd v280, v67
;; @0025                               v70 = isub v62, v7  ; v7 = 20
;; @0025                               v71 = uextend.i64 v70
;; @0025                               v72 = isub v69, v71
;; @0025                               store user2 little region1 v125, v72
;; @0025                               v79 = load.i32 user2 readonly region1 v53
;; @0025                               v73 = iconst.i32 1
;;                                     v211 = icmp ugt v79, v73  ; v73 = 1
;; @0025                               trapz v211, user17
;; @0025                               v82 = uextend.i64 v79
;;                                     v213 = ishl v82, v155  ; v155 = 2
;;                                     v283 = ushr v213, v281  ; v281 = 32
;; @0025                               trapnz v283, user2
;;                                     v220 = ishl v79, v193  ; v193 = 2
;; @0025                               v87 = uadd_overflow_trap v220, v7, user2  ; v7 = 20
;; @0025                               v91 = uadd_overflow_trap v44, v87, user2
;;                                     v124 = load.i32 notrap v151
;; @0025                               v92 = uextend.i64 v91
;; @0025                               v94 = iadd v280, v92
;;                                     v233 = iconst.i32 24
;; @0025                               v95 = isub v87, v233  ; v233 = 24
;; @0025                               v96 = uextend.i64 v95
;; @0025                               v97 = isub v94, v96
;; @0025                               store user2 little region1 v124, v97
;; @0025                               v104 = load.i32 user2 readonly region1 v53
;;                                     v239 = icmp ugt v104, v193  ; v193 = 2
;; @0025                               trapz v239, user17
;; @0025                               v107 = uextend.i64 v104
;;                                     v241 = ishl v107, v155  ; v155 = 2
;;                                     v284 = ushr v241, v281  ; v281 = 32
;; @0025                               trapnz v284, user2
;;                                     v248 = ishl v104, v193  ; v193 = 2
;; @0025                               v112 = uadd_overflow_trap v248, v7, user2  ; v7 = 20
;; @0025                               v116 = uadd_overflow_trap v44, v112, user2
;;                                     v123 = load.i32 notrap v150
;; @0025                               v117 = uextend.i64 v116
;; @0025                               v119 = iadd v280, v117
;;                                     v266 = iconst.i32 28
;; @0025                               v120 = isub v112, v266  ; v266 = 28
;; @0025                               v121 = uextend.i64 v120
;; @0025                               v122 = isub v119, v121
;; @0025                               store user2 little region1 v123, v122
;; @0029                               jump block1(v44)
;;
;;                                 block1(v5: i32):
;; @0029                               return v5
;; }
