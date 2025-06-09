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
;;                                     v135 = stack_addr.i64 ss2
;;                                     store notrap v2, v135
;;                                     v134 = stack_addr.i64 ss1
;;                                     store notrap v3, v134
;;                                     v133 = stack_addr.i64 ss0
;;                                     store notrap v4, v133
;; @0025                               v14 = iconst.i32 -1476395008
;; @0025                               v15 = iconst.i32 0
;;                                     v148 = iconst.i32 32
;; @0025                               v16 = iconst.i32 8
;; @0025                               v17 = call fn0(v0, v14, v15, v148, v16), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v14 = -1476395008, v15 = 0, v148 = 32, v16 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v129 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v18 = load.i64 notrap aligned readonly can_move v129+24
;; @0025                               v19 = uextend.i64 v17
;; @0025                               v20 = iadd v18, v19
;;                                     v128 = iconst.i64 16
;; @0025                               v21 = iadd v20, v128  ; v128 = 16
;; @0025                               store notrap aligned v6, v21  ; v6 = 3
;;                                     v91 = load.i32 notrap v135
;;                                     v126 = iconst.i32 1
;; @0025                               v26 = band v91, v126  ; v126 = 1
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
;;                                     v96 = iconst.i64 1
;; @0025                               v36 = iadd v35, v96  ; v96 = 1
;; @0025                               store notrap aligned v36, v34
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v87 = load.i32 notrap v135
;;                                     v150 = iconst.i64 20
;;                                     v156 = iadd.i64 v20, v150  ; v150 = 20
;; @0025                               store notrap aligned little v87, v156
;;                                     v86 = load.i32 notrap v134
;;                                     v180 = iconst.i32 1
;;                                     v181 = band v86, v180  ; v180 = 1
;;                                     v182 = iconst.i32 0
;;                                     v183 = icmp eq v86, v182  ; v182 = 0
;; @0025                               v45 = uextend.i32 v183
;; @0025                               v46 = bor v181, v45
;; @0025                               brif v46, block5, block4
;;
;;                                 block4:
;; @0025                               v47 = uextend.i64 v86
;; @0025                               v49 = iadd.i64 v18, v47
;;                                     v184 = iconst.i64 8
;; @0025                               v51 = iadd v49, v184  ; v184 = 8
;; @0025                               v52 = load.i64 notrap aligned v51
;;                                     v185 = iconst.i64 1
;; @0025                               v53 = iadd v52, v185  ; v185 = 1
;; @0025                               store notrap aligned v53, v51
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v82 = load.i32 notrap v134
;;                                     v158 = iconst.i64 24
;;                                     v164 = iadd.i64 v20, v158  ; v158 = 24
;; @0025                               store notrap aligned little v82, v164
;;                                     v81 = load.i32 notrap v133
;;                                     v186 = iconst.i32 1
;;                                     v187 = band v81, v186  ; v186 = 1
;;                                     v188 = iconst.i32 0
;;                                     v189 = icmp eq v81, v188  ; v188 = 0
;; @0025                               v62 = uextend.i32 v189
;; @0025                               v63 = bor v187, v62
;; @0025                               brif v63, block7, block6
;;
;;                                 block6:
;; @0025                               v64 = uextend.i64 v81
;; @0025                               v66 = iadd.i64 v18, v64
;;                                     v190 = iconst.i64 8
;; @0025                               v68 = iadd v66, v190  ; v190 = 8
;; @0025                               v69 = load.i64 notrap aligned v68
;;                                     v191 = iconst.i64 1
;; @0025                               v70 = iadd v69, v191  ; v191 = 1
;; @0025                               store notrap aligned v70, v68
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v77 = load.i32 notrap v133
;;                                     v166 = iconst.i64 28
;;                                     v172 = iadd.i64 v20, v166  ; v166 = 28
;; @0025                               store notrap aligned little v77, v172
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v17
;; }
