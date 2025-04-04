;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v129 = stack_addr.i64 ss2
;;                                     store notrap v2, v129
;;                                     v130 = stack_addr.i64 ss1
;;                                     store notrap v3, v130
;;                                     v131 = stack_addr.i64 ss0
;;                                     store notrap v4, v131
;; @0025                               v14 = iconst.i32 -1476395008
;; @0025                               v15 = iconst.i32 0
;;                                     v171 = iconst.i32 32
;; @0025                               v16 = iconst.i32 8
;; @0025                               v17 = call fn0(v0, v14, v15, v171, v16), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v14 = -1476395008, v15 = 0, v171 = 32, v16 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v19 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v20 = uextend.i64 v17
;; @0025                               v21 = iadd v19, v20
;;                                     v134 = iconst.i64 16
;; @0025                               v22 = iadd v21, v134  ; v134 = 16
;; @0025                               store notrap aligned v6, v22  ; v6 = 3
;;                                     v128 = load.i32 notrap v129
;;                                     v136 = iconst.i32 1
;; @0025                               v27 = band v128, v136  ; v136 = 1
;; @0025                               v28 = icmp eq v128, v15  ; v15 = 0
;; @0025                               v29 = uextend.i32 v28
;; @0025                               v30 = bor v27, v29
;; @0025                               brif v30, block3, block2
;;
;;                                 block2:
;; @0025                               v35 = uextend.i64 v128
;; @0025                               v94 = iconst.i64 8
;; @0025                               v37 = uadd_overflow_trap v35, v94, user1  ; v94 = 8
;; @0025                               v39 = uadd_overflow_trap v37, v94, user1  ; v94 = 8
;; @0025                               v92 = load.i64 notrap aligned readonly can_move v0+48
;; @0025                               v40 = icmp ule v39, v92
;; @0025                               trapz v40, user1
;; @0025                               v41 = iadd.i64 v19, v37
;; @0025                               v42 = load.i64 notrap aligned v41
;;                                     v156 = iconst.i64 1
;; @0025                               v43 = iadd v42, v156  ; v156 = 1
;; @0025                               store notrap aligned v43, v41
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v124 = load.i32 notrap v129
;;                                     v173 = iconst.i64 20
;;                                     v179 = iadd.i64 v21, v173  ; v173 = 20
;; @0025                               store notrap aligned little v124, v179
;;                                     v123 = load.i32 notrap v130
;;                                     v203 = iconst.i32 1
;;                                     v204 = band v123, v203  ; v203 = 1
;;                                     v205 = iconst.i32 0
;;                                     v206 = icmp eq v123, v205  ; v205 = 0
;; @0025                               v58 = uextend.i32 v206
;; @0025                               v59 = bor v204, v58
;; @0025                               brif v59, block5, block4
;;
;;                                 block4:
;; @0025                               v64 = uextend.i64 v123
;;                                     v207 = iconst.i64 8
;; @0025                               v66 = uadd_overflow_trap v64, v207, user1  ; v207 = 8
;; @0025                               v68 = uadd_overflow_trap v66, v207, user1  ; v207 = 8
;;                                     v208 = load.i64 notrap aligned readonly can_move v0+48
;; @0025                               v69 = icmp ule v68, v208
;; @0025                               trapz v69, user1
;; @0025                               v70 = iadd.i64 v19, v66
;; @0025                               v71 = load.i64 notrap aligned v70
;;                                     v209 = iconst.i64 1
;; @0025                               v72 = iadd v71, v209  ; v209 = 1
;; @0025                               store notrap aligned v72, v70
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v119 = load.i32 notrap v130
;;                                     v181 = iconst.i64 24
;;                                     v187 = iadd.i64 v21, v181  ; v181 = 24
;; @0025                               store notrap aligned little v119, v187
;;                                     v118 = load.i32 notrap v131
;;                                     v210 = iconst.i32 1
;;                                     v211 = band v118, v210  ; v210 = 1
;;                                     v212 = iconst.i32 0
;;                                     v213 = icmp eq v118, v212  ; v212 = 0
;; @0025                               v87 = uextend.i32 v213
;; @0025                               v88 = bor v211, v87
;; @0025                               brif v88, block7, block6
;;
;;                                 block6:
;; @0025                               v93 = uextend.i64 v118
;;                                     v214 = iconst.i64 8
;; @0025                               v95 = uadd_overflow_trap v93, v214, user1  ; v214 = 8
;; @0025                               v97 = uadd_overflow_trap v95, v214, user1  ; v214 = 8
;;                                     v215 = load.i64 notrap aligned readonly can_move v0+48
;; @0025                               v98 = icmp ule v97, v215
;; @0025                               trapz v98, user1
;; @0025                               v99 = iadd.i64 v19, v95
;; @0025                               v100 = load.i64 notrap aligned v99
;;                                     v216 = iconst.i64 1
;; @0025                               v101 = iadd v100, v216  ; v216 = 1
;; @0025                               store notrap aligned v101, v99
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v114 = load.i32 notrap v131
;;                                     v189 = iconst.i64 28
;;                                     v195 = iadd.i64 v21, v189  ; v189 = 28
;; @0025                               store notrap aligned little v114, v195
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v17
;; }
