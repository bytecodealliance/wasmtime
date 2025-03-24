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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u1:28 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v92 = stack_addr.i64 ss2
;;                                     store notrap v2, v92
;;                                     v93 = stack_addr.i64 ss1
;;                                     store notrap v3, v93
;;                                     v94 = stack_addr.i64 ss0
;;                                     store notrap v4, v94
;;                                     v146 = iconst.i64 0
;; @0025                               trapnz v146, user18  ; v146 = 0
;; @0025                               v7 = iconst.i32 20
;;                                     v147 = iconst.i32 12
;; @0025                               v12 = uadd_overflow_trap v7, v147, user18  ; v7 = 20, v147 = 12
;; @0025                               v14 = iconst.i32 -1476395008
;; @0025                               v15 = iconst.i32 0
;; @0025                               v16 = iconst.i32 8
;; @0025                               v17 = call fn0(v0, v14, v15, v12, v16), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v14 = -1476395008, v15 = 0, v16 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v97 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v18 = load.i64 notrap aligned readonly can_move v97+24
;; @0025                               v19 = uextend.i64 v17
;; @0025                               v20 = iadd v18, v19
;;                                     v99 = iconst.i64 16
;; @0025                               v21 = iadd v20, v99  ; v99 = 16
;; @0025                               store notrap aligned v6, v21  ; v6 = 3
;;                                     v91 = load.i32 notrap v92
;;                                     v101 = iconst.i32 1
;; @0025                               v26 = band v91, v101  ; v101 = 1
;; @0025                               v27 = icmp eq v91, v15  ; v15 = 0
;; @0025                               v28 = uextend.i32 v27
;; @0025                               v29 = bor v26, v28
;; @0025                               brif v29, block3, block2
;;
;;                                 block2:
;; @0025                               v30 = uextend.i64 v91
;; @0025                               v32 = iadd.i64 v18, v30
;; @0025                               v67 = iconst.i64 8
;; @0025                               v34 = iadd v32, v67  ; v67 = 8
;; @0025                               v35 = load.i64 notrap aligned v34
;;                                     v131 = iconst.i64 1
;; @0025                               v36 = iadd v35, v131  ; v131 = 1
;; @0025                               store notrap aligned v36, v34
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v87 = load.i32 notrap v92
;;                                     v149 = iconst.i64 20
;;                                     v155 = iadd.i64 v20, v149  ; v149 = 20
;; @0025                               store notrap aligned little v87, v155
;;                                     v86 = load.i32 notrap v93
;;                                     v179 = iconst.i32 1
;;                                     v180 = band v86, v179  ; v179 = 1
;;                                     v181 = iconst.i32 0
;;                                     v182 = icmp eq v86, v181  ; v181 = 0
;; @0025                               v45 = uextend.i32 v182
;; @0025                               v46 = bor v180, v45
;; @0025                               brif v46, block5, block4
;;
;;                                 block4:
;; @0025                               v47 = uextend.i64 v86
;; @0025                               v49 = iadd.i64 v18, v47
;;                                     v183 = iconst.i64 8
;; @0025                               v51 = iadd v49, v183  ; v183 = 8
;; @0025                               v52 = load.i64 notrap aligned v51
;;                                     v184 = iconst.i64 1
;; @0025                               v53 = iadd v52, v184  ; v184 = 1
;; @0025                               store notrap aligned v53, v51
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v82 = load.i32 notrap v93
;;                                     v157 = iconst.i64 24
;;                                     v163 = iadd.i64 v20, v157  ; v157 = 24
;; @0025                               store notrap aligned little v82, v163
;;                                     v81 = load.i32 notrap v94
;;                                     v185 = iconst.i32 1
;;                                     v186 = band v81, v185  ; v185 = 1
;;                                     v187 = iconst.i32 0
;;                                     v188 = icmp eq v81, v187  ; v187 = 0
;; @0025                               v62 = uextend.i32 v188
;; @0025                               v63 = bor v186, v62
;; @0025                               brif v63, block7, block6
;;
;;                                 block6:
;; @0025                               v64 = uextend.i64 v81
;; @0025                               v66 = iadd.i64 v18, v64
;;                                     v189 = iconst.i64 8
;; @0025                               v68 = iadd v66, v189  ; v189 = 8
;; @0025                               v69 = load.i64 notrap aligned v68
;;                                     v190 = iconst.i64 1
;; @0025                               v70 = iadd v69, v190  ; v190 = 1
;; @0025                               store notrap aligned v70, v68
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v77 = load.i32 notrap v94
;;                                     v165 = iconst.i64 28
;;                                     v171 = iadd.i64 v20, v165  ; v165 = 28
;; @0025                               store notrap aligned little v77, v171
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v17
;; }
