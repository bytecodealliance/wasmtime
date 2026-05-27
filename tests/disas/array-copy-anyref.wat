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
;;     ss0 = explicit_slot 4, align = 4
;;     ss1 = explicit_slot 4, align = 4
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
;;                                     v118 = stack_addr.i64 ss1
;;                                     store notrap v2, v118
;;                                     v117 = stack_addr.i64 ss0
;;                                     store notrap v4, v117
;; @002b                               trapz v2, user16
;; @002b                               v115 = load.i64 notrap aligned readonly can_move v0+8
;; @002b                               v8 = load.i64 notrap aligned readonly can_move v115+32
;; @002b                               v7 = uextend.i64 v2
;; @002b                               v9 = iadd v8, v7
;; @002b                               v10 = iconst.i64 16
;; @002b                               v11 = iadd v9, v10  ; v10 = 16
;; @002b                               v12 = load.i32 user2 readonly v11
;; @002b                               v14 = uextend.i64 v3
;; @002b                               v15 = uextend.i64 v6
;; @002b                               v17 = iadd v14, v15
;; @002b                               v13 = uextend.i64 v12
;; @002b                               v18 = icmp ugt v17, v13
;; @002b                               trapnz v18, user17
;; @002b                               trapz v4, user16
;; @002b                               v26 = uextend.i64 v4
;; @002b                               v28 = iadd v8, v26
;; @002b                               v30 = iadd v28, v10  ; v10 = 16
;; @002b                               v31 = load.i32 user2 readonly v30
;; @002b                               v33 = uextend.i64 v5
;; @002b                               v36 = iadd v33, v15
;; @002b                               v32 = uextend.i64 v31
;; @002b                               v37 = icmp ugt v36, v32
;; @002b                               trapnz v37, user17
;; @002b                               v49 = load.i64 notrap aligned v115+40
;;                                     v111 = iconst.i64 20
;; @002b                               v22 = iadd v9, v111  ; v111 = 20
;;                                     v121 = iconst.i64 2
;;                                     v122 = ishl v14, v121  ; v121 = 2
;; @002b                               v25 = iadd v22, v122
;;                                     v126 = ishl v15, v121  ; v121 = 2
;; @002b                               v51 = uadd_overflow_trap v25, v126, user2
;; @002b                               v50 = iadd v8, v49
;; @002b                               v52 = icmp ugt v51, v50
;; @002b                               trapnz v52, user2
;; @002b                               v41 = iadd v28, v111  ; v111 = 20
;;                                     v124 = ishl v33, v121  ; v121 = 2
;; @002b                               v44 = iadd v41, v124
;; @002b                               v56 = uadd_overflow_trap v44, v126, user2
;; @002b                               v57 = icmp ugt v56, v50
;; @002b                               trapnz v57, user2
;; @002b                               brif v15, block2, block5
;;
;;                                 block2:
;; @002b                               v58 = icmp.i64 ult v25, v44
;; @002b                               v61 = iadd.i64 v25, v126
;; @002b                               v62 = iadd.i64 v44, v126
;; @002b                               v64 = iadd.i32 v5, v6
;;                                     v110 = iconst.i64 4
;; @002b                               v84 = iconst.i32 1
;; @002b                               brif v58, block3(v25, v44, v5), block4(v61, v62, v64)
;;
;;                                 block3(v65: i64, v66: i64, v67: i32):
;; @002b                               v70 = load.i32 user2 little v66
;; @002b                               store user2 little v70, v65
;;                                     v133 = iconst.i64 4
;;                                     v134 = iadd v66, v133  ; v133 = 4
;; @002b                               v74 = icmp eq v134, v62
;;                                     v135 = iadd v65, v133  ; v133 = 4
;;                                     v136 = iconst.i32 1
;;                                     v137 = iadd v67, v136  ; v136 = 1
;; @002b                               brif v74, block5, block3(v135, v134, v137)
;;
;;                                 block4(v75: i64, v76: i64, v77: i32):
;;                                     v128 = iconst.i64 4
;;                                     v129 = isub v76, v128  ; v128 = 4
;; @002b                               v86 = load.i32 user2 little v129
;;                                     v130 = isub v75, v128  ; v128 = 4
;; @002b                               store user2 little v86, v130
;; @002b                               v87 = icmp eq v129, v44
;;                                     v131 = iconst.i32 1
;;                                     v132 = isub v77, v131  ; v131 = 1
;; @002b                               brif v87, block5, block4(v130, v129, v132)
;;
;;                                 block5:
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
