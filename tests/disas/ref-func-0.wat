;;! target = "x86_64"

(module
  (func $imported (import "env" "f") (param i32) (result i32))
  (func $local (result externref externref funcref funcref)
    global.get 0
    global.get 1
    global.get 2
    global.get 3)

  (global (export "externref-imported") externref (ref.null extern))
  (global (export "externref-local") externref (ref.null extern))
  (global (export "funcref-imported") funcref (ref.func $imported))
  (global (export "funcref-local") funcref (ref.func $local)))

;; function u0:0(i64 vmctx, i64) -> i32, i32, i64, i64 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     ss1 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:45 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v189 = iconst.i64 80
;; @008f                               v7 = iadd v0, v189  ; v189 = 80
;; @008f                               v8 = load.i32 notrap aligned readonly can_move v7
;;                                     v188 = stack_addr.i64 ss0
;;                                     store notrap v8, v188
;;                                     v187 = stack_addr.i64 ss0
;;                                     v139 = load.i32 notrap v187
;;                                     v186 = iconst.i32 1
;; @008f                               v9 = band v139, v186  ; v186 = 1
;;                                     v185 = stack_addr.i64 ss0
;;                                     v138 = load.i32 notrap v185
;;                                     v184 = iconst.i32 0
;; @008f                               v10 = icmp eq v138, v184  ; v184 = 0
;; @008f                               v11 = uextend.i32 v10
;; @008f                               v12 = bor v9, v11
;; @008f                               brif v12, block4, block2
;;
;;                                 block2:
;;                                     v183 = stack_addr.i64 ss0
;;                                     v137 = load.i32 notrap v183
;; @008f                               v13 = uextend.i64 v137
;; @008f                               v181 = load.i64 notrap aligned readonly can_move v0+8
;; @008f                               v14 = load.i64 notrap aligned readonly can_move v181+32
;; @008f                               v15 = iadd v14, v13
;; @008f                               v16 = load.i32 user2 v15
;; @008f                               v17 = iconst.i32 2
;; @008f                               v18 = band v16, v17  ; v17 = 2
;; @008f                               brif v18, block4, block3
;;
;;                                 block3:
;; @008f                               v20 = load.i64 notrap aligned readonly can_move v0+32
;; @008f                               v21 = load.i32 user2 v20
;;                                     v180 = stack_addr.i64 ss0
;;                                     v136 = load.i32 notrap v180
;; @008f                               v22 = uextend.i64 v136
;; @008f                               v178 = load.i64 notrap aligned readonly can_move v0+8
;; @008f                               v23 = load.i64 notrap aligned readonly can_move v178+32
;; @008f                               v24 = iadd v23, v22
;; @008f                               v25 = iconst.i64 16
;; @008f                               v26 = iadd v24, v25  ; v25 = 16
;; @008f                               store user2 v21, v26
;; @008f                               v27 = iconst.i32 2
;; @008f                               v28 = bor.i32 v16, v27  ; v27 = 2
;;                                     v177 = stack_addr.i64 ss0
;;                                     v135 = load.i32 notrap v177
;; @008f                               v29 = uextend.i64 v135
;; @008f                               v175 = load.i64 notrap aligned readonly can_move v0+8
;; @008f                               v30 = load.i64 notrap aligned readonly can_move v175+32
;; @008f                               v31 = iadd v30, v29
;; @008f                               store user2 v28, v31
;;                                     v174 = stack_addr.i64 ss0
;;                                     v134 = load.i32 notrap v174
;; @008f                               v32 = uextend.i64 v134
;; @008f                               v172 = load.i64 notrap aligned readonly can_move v0+8
;; @008f                               v33 = load.i64 notrap aligned readonly can_move v172+32
;; @008f                               v34 = iadd v33, v32
;; @008f                               v35 = iconst.i64 8
;; @008f                               v36 = iadd v34, v35  ; v35 = 8
;; @008f                               v37 = load.i64 user2 v36
;;                                     v171 = iconst.i64 1
;; @008f                               v38 = iadd v37, v171  ; v171 = 1
;;                                     v170 = stack_addr.i64 ss0
;;                                     v133 = load.i32 notrap v170
;; @008f                               v39 = uextend.i64 v133
;; @008f                               v168 = load.i64 notrap aligned readonly can_move v0+8
;; @008f                               v40 = load.i64 notrap aligned readonly can_move v168+32
;; @008f                               v41 = iadd v40, v39
;; @008f                               v42 = iconst.i64 8
;; @008f                               v43 = iadd v41, v42  ; v42 = 8
;; @008f                               store user2 v38, v43
;;                                     v167 = stack_addr.i64 ss0
;;                                     v132 = load.i32 notrap v167
;; @008f                               store user2 v132, v20
;; @008f                               v45 = load.i64 notrap aligned readonly can_move v0+32
;; @008f                               v46 = load.i32 notrap aligned v45+4
;;                                     v166 = iconst.i32 1
;; @008f                               v47 = iadd v46, v166  ; v166 = 1
;; @008f                               v49 = load.i64 notrap aligned readonly can_move v0+32
;; @008f                               store notrap aligned v47, v49+4
;; @008f                               v51 = load.i64 notrap aligned readonly can_move v0+32
;; @008f                               v52 = load.i32 notrap aligned v51+4
;; @008f                               v54 = load.i64 notrap aligned readonly can_move v0+32
;; @008f                               v55 = load.i32 notrap aligned v54+8
;; @008f                               v56 = iadd v55, v55
;; @008f                               v57 = iconst.i32 1024
;; @008f                               v58 = umax v56, v57  ; v57 = 1024
;; @008f                               v59 = icmp uge v52, v58
;; @008f                               brif v59, block5, block6
;;
;;                                 block5 cold:
;; @008f                               v61 = call fn0(v0), stack_map=[i32 @ ss0+0]
;; @008f                               jump block6
;;
;;                                 block6:
;; @008f                               jump block4
;;
;;                                 block4:
;;                                     v165 = iconst.i64 96
;; @0091                               v63 = iadd.i64 v0, v165  ; v165 = 96
;; @0091                               v64 = load.i32 notrap aligned readonly can_move v63
;;                                     v164 = stack_addr.i64 ss1
;;                                     store notrap v64, v164
;;                                     v163 = stack_addr.i64 ss1
;;                                     v131 = load.i32 notrap v163
;;                                     v162 = iconst.i32 1
;; @0091                               v65 = band v131, v162  ; v162 = 1
;;                                     v161 = stack_addr.i64 ss1
;;                                     v130 = load.i32 notrap v161
;;                                     v160 = iconst.i32 0
;; @0091                               v66 = icmp eq v130, v160  ; v160 = 0
;; @0091                               v67 = uextend.i32 v66
;; @0091                               v68 = bor v65, v67
;; @0091                               brif v68, block9, block7
;;
;;                                 block7:
;;                                     v159 = stack_addr.i64 ss1
;;                                     v129 = load.i32 notrap v159
;; @0091                               v69 = uextend.i64 v129
;; @0091                               v157 = load.i64 notrap aligned readonly can_move v0+8
;; @0091                               v70 = load.i64 notrap aligned readonly can_move v157+32
;; @0091                               v71 = iadd v70, v69
;; @0091                               v72 = load.i32 user2 v71
;; @0091                               v73 = iconst.i32 2
;; @0091                               v74 = band v72, v73  ; v73 = 2
;; @0091                               brif v74, block9, block8
;;
;;                                 block8:
;; @0091                               v76 = load.i64 notrap aligned readonly can_move v0+32
;; @0091                               v77 = load.i32 user2 v76
;;                                     v156 = stack_addr.i64 ss1
;;                                     v128 = load.i32 notrap v156
;; @0091                               v78 = uextend.i64 v128
;; @0091                               v154 = load.i64 notrap aligned readonly can_move v0+8
;; @0091                               v79 = load.i64 notrap aligned readonly can_move v154+32
;; @0091                               v80 = iadd v79, v78
;; @0091                               v81 = iconst.i64 16
;; @0091                               v82 = iadd v80, v81  ; v81 = 16
;; @0091                               store user2 v77, v82
;; @0091                               v83 = iconst.i32 2
;; @0091                               v84 = bor.i32 v72, v83  ; v83 = 2
;;                                     v153 = stack_addr.i64 ss1
;;                                     v127 = load.i32 notrap v153
;; @0091                               v85 = uextend.i64 v127
;; @0091                               v151 = load.i64 notrap aligned readonly can_move v0+8
;; @0091                               v86 = load.i64 notrap aligned readonly can_move v151+32
;; @0091                               v87 = iadd v86, v85
;; @0091                               store user2 v84, v87
;;                                     v150 = stack_addr.i64 ss1
;;                                     v126 = load.i32 notrap v150
;; @0091                               v88 = uextend.i64 v126
;; @0091                               v148 = load.i64 notrap aligned readonly can_move v0+8
;; @0091                               v89 = load.i64 notrap aligned readonly can_move v148+32
;; @0091                               v90 = iadd v89, v88
;; @0091                               v91 = iconst.i64 8
;; @0091                               v92 = iadd v90, v91  ; v91 = 8
;; @0091                               v93 = load.i64 user2 v92
;;                                     v147 = iconst.i64 1
;; @0091                               v94 = iadd v93, v147  ; v147 = 1
;;                                     v146 = stack_addr.i64 ss1
;;                                     v125 = load.i32 notrap v146
;; @0091                               v95 = uextend.i64 v125
;; @0091                               v144 = load.i64 notrap aligned readonly can_move v0+8
;; @0091                               v96 = load.i64 notrap aligned readonly can_move v144+32
;; @0091                               v97 = iadd v96, v95
;; @0091                               v98 = iconst.i64 8
;; @0091                               v99 = iadd v97, v98  ; v98 = 8
;; @0091                               store user2 v94, v99
;;                                     v143 = stack_addr.i64 ss1
;;                                     v124 = load.i32 notrap v143
;; @0091                               store user2 v124, v76
;; @0091                               v101 = load.i64 notrap aligned readonly can_move v0+32
;; @0091                               v102 = load.i32 notrap aligned v101+4
;;                                     v142 = iconst.i32 1
;; @0091                               v103 = iadd v102, v142  ; v142 = 1
;; @0091                               v105 = load.i64 notrap aligned readonly can_move v0+32
;; @0091                               store notrap aligned v103, v105+4
;; @0091                               v107 = load.i64 notrap aligned readonly can_move v0+32
;; @0091                               v108 = load.i32 notrap aligned v107+4
;; @0091                               v110 = load.i64 notrap aligned readonly can_move v0+32
;; @0091                               v111 = load.i32 notrap aligned v110+8
;; @0091                               v112 = iadd v111, v111
;; @0091                               v113 = iconst.i32 1024
;; @0091                               v114 = umax v112, v113  ; v113 = 1024
;; @0091                               v115 = icmp uge v108, v114
;; @0091                               brif v115, block10, block11
;;
;;                                 block10 cold:
;; @0091                               v117 = call fn0(v0), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0091                               jump block11
;;
;;                                 block11:
;; @0091                               jump block9
;;
;;                                 block9:
;; @0093                               v119 = load.i64 notrap aligned table v0+112
;; @0095                               v121 = load.i64 notrap aligned table v0+128
;;                                     v141 = stack_addr.i64 ss0
;;                                     v122 = load.i32 notrap v141
;;                                     v140 = stack_addr.i64 ss1
;;                                     v123 = load.i32 notrap v140
;; @0097                               jump block1
;;
;;                                 block1:
;; @0097                               return v122, v123, v119, v121
;; }
