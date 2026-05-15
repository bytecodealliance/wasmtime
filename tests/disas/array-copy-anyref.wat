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
;; @002b                               v83 = load.i64 notrap aligned readonly can_move v0+8
;; @002b                               v8 = load.i64 notrap aligned readonly can_move v83+32
;; @002b                               v7 = uextend.i64 v4
;; @002b                               v9 = iadd v8, v7
;; @002b                               v10 = iconst.i64 16
;; @002b                               v11 = iadd v9, v10  ; v10 = 16
;; @002b                               v12 = load.i32 user2 readonly v11
;; @002b                               v13 = uadd_overflow_trap v5, v6, user17
;; @002b                               v14 = icmp ugt v13, v12
;; @002b                               trapnz v14, user17
;; @002b                               v16 = uextend.i64 v12
;;                                     v85 = iconst.i64 2
;;                                     v86 = ishl v16, v85  ; v85 = 2
;;                                     v82 = iconst.i64 32
;; @002b                               v18 = ushr v86, v82  ; v82 = 32
;; @002b                               trapnz v18, user2
;;                                     v95 = iconst.i32 2
;;                                     v96 = ishl v12, v95  ; v95 = 2
;; @002b                               v20 = iconst.i32 20
;; @002b                               v21 = uadd_overflow_trap v96, v20, user2  ; v20 = 20
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
;;                                     v106 = ishl v41, v85  ; v85 = 2
;; @002b                               v43 = ushr v106, v82  ; v82 = 32
;; @002b                               trapnz v43, user2
;;                                     v113 = ishl v37, v95  ; v95 = 2
;; @002b                               v46 = uadd_overflow_trap v113, v20, user2  ; v20 = 20
;; @002b                               v50 = uadd_overflow_trap v2, v46, user2
;; @002b                               brif v6, block2, block5
;;
;;                                 block2:
;; @002b                               v51 = uextend.i64 v50
;; @002b                               v53 = iadd.i64 v8, v51
;;                                     v129 = iconst.i32 2
;;                                     v130 = ishl.i32 v3, v129  ; v129 = 2
;;                                     v131 = iconst.i32 20
;;                                     v132 = iadd v130, v131  ; v131 = 20
;; @002b                               v54 = isub.i32 v46, v132
;; @002b                               v55 = uextend.i64 v54
;; @002b                               v56 = isub v53, v55
;; @002b                               v26 = uextend.i64 v25
;; @002b                               v28 = iadd.i64 v8, v26
;;                                     v133 = ishl.i32 v5, v129  ; v129 = 2
;;                                     v134 = iadd v133, v131  ; v131 = 20
;; @002b                               v29 = isub.i32 v21, v134
;; @002b                               v30 = uextend.i64 v29
;; @002b                               v31 = isub v28, v30
;; @002b                               v62 = icmp ult v56, v31
;;                                     v135 = ishl.i32 v6, v129  ; v129 = 2
;; @002b                               v58 = uextend.i64 v135
;; @002b                               v60 = iadd v56, v58
;; @002b                               v59 = iadd v31, v58
;; @002b                               v15 = iconst.i64 4
;;                                     v126 = iadd v28, v15  ; v15 = 4
;; @002b                               brif v62, block3(v56, v31), block4(v60, v59)
;;
;;                                 block3(v63: i64, v64: i64):
;; @002b                               v65 = load.i32 user2 little v64
;; @002b                               store user2 little v65, v63
;;                                     v141 = iconst.i64 4
;;                                     v142 = iadd v64, v141  ; v141 = 4
;; @002b                               v68 = icmp eq v142, v59
;;                                     v143 = iadd v63, v141  ; v141 = 4
;; @002b                               brif v68, block5, block3(v143, v142)
;;
;;                                 block4(v69: i64, v70: i64):
;;                                     v136 = iconst.i64 4
;;                                     v137 = isub v70, v136  ; v136 = 4
;; @002b                               v73 = load.i32 user2 little v137
;;                                     v138 = isub v69, v136  ; v136 = 4
;; @002b                               store user2 little v73, v138
;;                                     v125 = iadd v70, v30
;;                                     v139 = iadd.i64 v28, v15  ; v15 = 4
;;                                     v140 = icmp eq v125, v139
;; @002b                               brif v140, block5, block4(v138, v137)
;;
;;                                 block5:
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
