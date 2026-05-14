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
;; @002b                               v84 = load.i64 notrap aligned readonly can_move v0+8
;; @002b                               v11 = load.i64 notrap aligned readonly can_move v84+32
;; @002b                               v10 = uextend.i64 v4
;; @002b                               v12 = iadd v11, v10
;; @002b                               v13 = iconst.i64 16
;; @002b                               v14 = iadd v12, v13  ; v13 = 16
;; @002b                               v15 = load.i32 user2 readonly v14
;; @002b                               v16 = uadd_overflow_trap v5, v6, user17
;; @002b                               v17 = icmp ugt v16, v15
;; @002b                               trapnz v17, user17
;; @002b                               v19 = uextend.i64 v15
;;                                     v89 = iconst.i64 2
;;                                     v90 = ishl v19, v89  ; v89 = 2
;;                                     v83 = iconst.i64 32
;; @002b                               v21 = ushr v90, v83  ; v83 = 32
;; @002b                               trapnz v21, user2
;;                                     v86 = iconst.i32 2
;;                                     v98 = ishl v15, v86  ; v86 = 2
;; @002b                               v23 = iconst.i32 20
;; @002b                               v24 = uadd_overflow_trap v98, v23, user2  ; v23 = 20
;; @002b                               v28 = uadd_overflow_trap v4, v24, user2
;; @002b                               trapz v2, user16
;; @002b                               v36 = uextend.i64 v2
;; @002b                               v38 = iadd v11, v36
;; @002b                               v40 = iadd v38, v13  ; v13 = 16
;; @002b                               v41 = load.i32 user2 readonly v40
;; @002b                               v42 = uadd_overflow_trap v3, v6, user17
;; @002b                               v43 = icmp ugt v42, v41
;; @002b                               trapnz v43, user17
;; @002b                               v45 = uextend.i64 v41
;;                                     v108 = ishl v45, v89  ; v89 = 2
;; @002b                               v47 = ushr v108, v83  ; v83 = 32
;; @002b                               trapnz v47, user2
;;                                     v115 = ishl v41, v86  ; v86 = 2
;; @002b                               v50 = uadd_overflow_trap v115, v23, user2  ; v23 = 20
;; @002b                               v54 = uadd_overflow_trap v2, v50, user2
;; @002b                               brif v6, block2, block5
;;
;;                                 block2:
;; @002b                               v55 = uextend.i64 v54
;; @002b                               v57 = iadd.i64 v11, v55
;;                                     v129 = iconst.i32 2
;;                                     v130 = ishl.i32 v3, v129  ; v129 = 2
;;                                     v131 = iconst.i32 20
;;                                     v132 = iadd v130, v131  ; v131 = 20
;; @002b                               v58 = isub.i32 v50, v132
;; @002b                               v59 = uextend.i64 v58
;; @002b                               v60 = isub v57, v59
;; @002b                               v29 = uextend.i64 v28
;; @002b                               v31 = iadd.i64 v11, v29
;;                                     v133 = ishl.i32 v5, v129  ; v129 = 2
;;                                     v134 = iadd v133, v131  ; v131 = 20
;; @002b                               v32 = isub.i32 v24, v134
;; @002b                               v33 = uextend.i64 v32
;; @002b                               v34 = isub v31, v33
;; @002b                               v63 = icmp ult v60, v34
;;                                     v135 = ishl.i32 v6, v129  ; v129 = 2
;; @002b                               v9 = uextend.i64 v135
;; @002b                               v61 = iadd v60, v9
;; @002b                               v35 = iadd v34, v9
;; @002b                               v18 = iconst.i64 4
;;                                     v126 = iadd v31, v18  ; v18 = 4
;; @002b                               brif v63, block3(v60, v34), block4(v61, v35)
;;
;;                                 block3(v64: i64, v65: i64):
;; @002b                               v66 = load.i32 user2 little v65
;; @002b                               store user2 little v66, v64
;;                                     v141 = iconst.i64 4
;;                                     v142 = iadd v65, v141  ; v141 = 4
;; @002b                               v69 = icmp eq v142, v35
;;                                     v143 = iadd v64, v141  ; v141 = 4
;; @002b                               brif v69, block5, block3(v143, v142)
;;
;;                                 block4(v70: i64, v71: i64):
;;                                     v136 = iconst.i64 4
;;                                     v137 = isub v71, v136  ; v136 = 4
;; @002b                               v74 = load.i32 user2 little v137
;;                                     v138 = isub v70, v136  ; v136 = 4
;; @002b                               store user2 little v74, v138
;;                                     v125 = iadd v71, v33
;;                                     v139 = iadd.i64 v31, v18  ; v18 = 4
;;                                     v140 = icmp eq v125, v139
;; @002b                               brif v140, block5, block4(v138, v137)
;;
;;                                 block5:
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
