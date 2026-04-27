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
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v137 = stack_addr.i64 ss2
;;                                     store notrap v2, v137
;;                                     v136 = stack_addr.i64 ss1
;;                                     store notrap v3, v136
;;                                     v135 = stack_addr.i64 ss0
;;                                     store notrap v4, v135
;; @0025                               v14 = iconst.i32 -1476395008
;; @0025                               v16 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v17 = load.i32 notrap aligned readonly can_move v16
;;                                     v149 = iconst.i32 40
;; @0025                               v18 = iconst.i32 8
;; @0025                               v19 = call fn0(v0, v14, v17, v149, v18), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v14 = -1476395008, v149 = 40, v18 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v131 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v20 = load.i64 notrap aligned readonly can_move v131+32
;; @0025                               v21 = uextend.i64 v19
;; @0025                               v22 = iadd v20, v21
;;                                     v130 = iconst.i64 24
;; @0025                               v23 = iadd v22, v130  ; v130 = 24
;; @0025                               store notrap aligned v6, v23  ; v6 = 3
;;                                     v93 = load.i32 notrap v137
;;                                     v128 = iconst.i32 1
;; @0025                               v28 = band v93, v128  ; v128 = 1
;;                                     v126 = iconst.i32 0
;; @0025                               v29 = icmp eq v93, v126  ; v126 = 0
;; @0025                               v30 = uextend.i32 v29
;; @0025                               v31 = bor v28, v30
;; @0025                               brif v31, block3, block2
;;
;;                                 block2:
;; @0025                               v32 = uextend.i64 v93
;; @0025                               v34 = iadd.i64 v20, v32
;;                                     v165 = iconst.i64 8
;; @0025                               v36 = iadd v34, v165  ; v165 = 8
;; @0025                               v37 = load.i64 notrap aligned v36
;;                                     v98 = iconst.i64 1
;; @0025                               v38 = iadd v37, v98  ; v98 = 1
;; @0025                               store notrap aligned v38, v36
;; @0025                               jump block3
;;
;;                                 block3:
;;                                     v89 = load.i32 notrap v137
;;                                     v151 = iconst.i64 28
;;                                     v156 = iadd.i64 v22, v151  ; v151 = 28
;; @0025                               store notrap aligned little v89, v156
;;                                     v88 = load.i32 notrap v136
;;                                     v243 = iconst.i32 1
;;                                     v244 = band v88, v243  ; v243 = 1
;;                                     v245 = iconst.i32 0
;;                                     v246 = icmp eq v88, v245  ; v245 = 0
;; @0025                               v47 = uextend.i32 v246
;; @0025                               v48 = bor v244, v47
;; @0025                               brif v48, block5, block4
;;
;;                                 block4:
;; @0025                               v49 = uextend.i64 v88
;; @0025                               v51 = iadd.i64 v20, v49
;;                                     v247 = iconst.i64 8
;; @0025                               v53 = iadd v51, v247  ; v247 = 8
;; @0025                               v54 = load.i64 notrap aligned v53
;;                                     v248 = iconst.i64 1
;; @0025                               v55 = iadd v54, v248  ; v248 = 1
;; @0025                               store notrap aligned v55, v53
;; @0025                               jump block5
;;
;;                                 block5:
;;                                     v84 = load.i32 notrap v136
;;                                     v133 = iconst.i64 32
;;                                     v163 = iadd.i64 v22, v133  ; v133 = 32
;; @0025                               store notrap aligned little v84, v163
;;                                     v83 = load.i32 notrap v135
;;                                     v249 = iconst.i32 1
;;                                     v250 = band v83, v249  ; v249 = 1
;;                                     v251 = iconst.i32 0
;;                                     v252 = icmp eq v83, v251  ; v251 = 0
;; @0025                               v64 = uextend.i32 v252
;; @0025                               v65 = bor v250, v64
;; @0025                               brif v65, block7, block6
;;
;;                                 block6:
;; @0025                               v66 = uextend.i64 v83
;; @0025                               v68 = iadd.i64 v20, v66
;;                                     v253 = iconst.i64 8
;; @0025                               v70 = iadd v68, v253  ; v253 = 8
;; @0025                               v71 = load.i64 notrap aligned v70
;;                                     v254 = iconst.i64 1
;; @0025                               v72 = iadd v71, v254  ; v254 = 1
;; @0025                               store notrap aligned v72, v70
;; @0025                               jump block7
;;
;;                                 block7:
;;                                     v79 = load.i32 notrap v135
;;                                     v179 = iconst.i64 36
;;                                     v184 = iadd.i64 v22, v179  ; v179 = 36
;; @0025                               store notrap aligned little v79, v184
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
