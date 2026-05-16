;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc -Ccollector=copying'

(module
  (type $a (array (mut anyref)))

  (func $copy (param (ref $a) i32 (ref $a) i32 i32)
    (array.copy $a $a (local.get 0) (local.get 1) (local.get 2) (local.get 3) (local.get 4))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32):
;; @002b                               trapz v4, user16
;; @002b                               v93 = load.i64 notrap aligned readonly can_move v0+8
;; @002b                               v8 = load.i64 notrap aligned readonly can_move v93+32
;; @002b                               v7 = uextend.i64 v4
;; @002b                               v9 = iadd v8, v7
;; @002b                               v10 = iconst.i64 16
;; @002b                               v11 = iadd v9, v10  ; v10 = 16
;; @002b                               v12 = load.i32 user2 readonly v11
;; @002b                               v13 = uadd_overflow_trap v5, v6, user17
;; @002b                               v14 = icmp ugt v13, v12
;; @002b                               trapnz v14, user17
;; @002b                               v16 = uextend.i64 v12
;;                                     v95 = iconst.i64 2
;;                                     v96 = ishl v16, v95  ; v95 = 2
;;                                     v92 = iconst.i64 32
;; @002b                               v18 = ushr v96, v92  ; v92 = 32
;; @002b                               trapnz v18, user2
;;                                     v105 = iconst.i32 2
;;                                     v106 = ishl v12, v105  ; v105 = 2
;; @002b                               v20 = iconst.i32 20
;; @002b                               v21 = uadd_overflow_trap v106, v20, user2  ; v20 = 20
;; @002b                               v25 = uadd_overflow_trap v4, v21, user2
;; @002b                               trapz v2, user16
;; @002b                               v32 = uextend.i64 v2
;; @002b                               v34 = iadd v8, v32
;; @002b                               v36 = iadd v34, v10  ; v10 = 16
;; @002b                               v37 = load.i32 user2 readonly v36
;; @002b                               v38 = uadd_overflow_trap v3, v6, user17
;; @002b                               v39 = icmp ugt v38, v37
;; @002b                               trapnz v39, user17
;; @002b                               v41 = uextend.i64 v37
;;                                     v116 = ishl v41, v95  ; v95 = 2
;; @002b                               v43 = ushr v116, v92  ; v92 = 32
;; @002b                               trapnz v43, user2
;;                                     v123 = ishl v37, v105  ; v105 = 2
;; @002b                               v46 = uadd_overflow_trap v123, v20, user2  ; v20 = 20
;; @002b                               v50 = uadd_overflow_trap v2, v46, user2
;; @002b                               v60 = uextend.i64 v6
;; @002b                               brif v60, block2, block5
;;
;;                                 block2:
;; @002b                               v51 = uextend.i64 v50
;; @002b                               v53 = iadd.i64 v8, v51
;;                                     v141 = iconst.i32 2
;;                                     v142 = ishl.i32 v3, v141  ; v141 = 2
;;                                     v143 = iconst.i32 20
;;                                     v144 = iadd v142, v143  ; v143 = 20
;; @002b                               v54 = isub.i32 v46, v144
;; @002b                               v55 = uextend.i64 v54
;; @002b                               v56 = isub v53, v55
;; @002b                               v26 = uextend.i64 v25
;; @002b                               v28 = iadd.i64 v8, v26
;;                                     v145 = ishl.i32 v5, v141  ; v141 = 2
;;                                     v146 = iadd v145, v143  ; v143 = 20
;; @002b                               v29 = isub.i32 v21, v146
;; @002b                               v30 = uextend.i64 v29
;; @002b                               v31 = isub v28, v30
;; @002b                               v61 = icmp ult v56, v31
;;                                     v147 = iconst.i64 2
;;                                     v148 = ishl.i64 v60, v147  ; v147 = 2
;; @002b                               v63 = iadd v56, v148
;; @002b                               v64 = iadd v31, v148
;; @002b                               v66 = iadd.i32 v5, v6
;; @002b                               v15 = iconst.i64 4
;;                                     v138 = iadd v28, v15  ; v15 = 4
;; @002b                               v80 = iconst.i32 1
;; @002b                               brif v61, block3(v56, v31, v5), block4(v63, v64, v66)
;;
;;                                 block3(v67: i64, v68: i64, v69: i32):
;; @002b                               v70 = load.i32 user2 little v68
;; @002b                               store user2 little v70, v67
;;                                     v156 = iconst.i64 4
;;                                     v157 = iadd v68, v156  ; v156 = 4
;; @002b                               v74 = icmp eq v157, v64
;;                                     v158 = iadd v67, v156  ; v156 = 4
;;                                     v159 = iconst.i32 1
;;                                     v160 = iadd v69, v159  ; v159 = 1
;; @002b                               brif v74, block5, block3(v158, v157, v160)
;;
;;                                 block4(v75: i64, v76: i64, v77: i32):
;;                                     v149 = iconst.i64 4
;;                                     v150 = isub v76, v149  ; v149 = 4
;; @002b                               v82 = load.i32 user2 little v150
;;                                     v151 = isub v75, v149  ; v149 = 4
;; @002b                               store user2 little v82, v151
;;                                     v137 = iadd v76, v30
;;                                     v152 = iadd.i64 v28, v15  ; v15 = 4
;;                                     v153 = icmp eq v137, v152
;;                                     v154 = iconst.i32 1
;;                                     v155 = isub v77, v154  ; v154 = 1
;; @002b                               brif v153, block5, block4(v151, v150, v155)
;;
;;                                 block5:
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
