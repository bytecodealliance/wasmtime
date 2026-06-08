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
;;     region0 = 2147483648 "GcHeap"
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
;; @002b                               trapz v2, user16
;; @002b                               v115 = load.i64 notrap aligned readonly can_move v0+8
;; @002b                               v8 = load.i64 notrap aligned readonly can_move v115+32
;; @002b                               v7 = uextend.i64 v2
;; @002b                               v9 = iadd v8, v7
;; @002b                               v10 = iconst.i64 16
;; @002b                               v11 = iadd v9, v10  ; v10 = 16
;; @002b                               v12 = load.i32 user2 readonly region0 v11
;; @002b                               v14 = uextend.i64 v3
;; @002b                               v15 = uextend.i64 v6
;; @002b                               v18 = iadd v14, v15
;; @002b                               v13 = uextend.i64 v12
;; @002b                               v19 = icmp ugt v18, v13
;; @002b                               trapnz v19, user17
;; @002b                               trapz v4, user16
;; @002b                               v29 = uextend.i64 v4
;; @002b                               v31 = iadd v8, v29
;; @002b                               v33 = iadd v31, v10  ; v10 = 16
;; @002b                               v34 = load.i32 user2 readonly region0 v33
;; @002b                               v36 = uextend.i64 v5
;; @002b                               v40 = iadd v36, v15
;; @002b                               v35 = uextend.i64 v34
;; @002b                               v41 = icmp ugt v40, v35
;; @002b                               trapnz v41, user17
;; @002b                               v57 = load.i64 notrap aligned v115+40
;; @002b                               v23 = iconst.i64 20
;; @002b                               v24 = iadd v9, v23  ; v23 = 20
;;                                     v119 = iconst.i64 2
;;                                     v120 = ishl v14, v119  ; v119 = 2
;; @002b                               v28 = iadd v24, v120
;;                                     v124 = ishl v15, v119  ; v119 = 2
;; @002b                               v59 = uadd_overflow_trap v28, v124, user2
;; @002b                               v58 = iadd v8, v57
;; @002b                               v60 = icmp ugt v59, v58
;; @002b                               trapnz v60, user2
;; @002b                               v46 = iadd v31, v23  ; v23 = 20
;;                                     v122 = ishl v36, v119  ; v119 = 2
;; @002b                               v50 = iadd v46, v122
;; @002b                               v64 = uadd_overflow_trap v50, v124, user2
;; @002b                               v65 = icmp ugt v64, v58
;; @002b                               trapnz v65, user2
;; @002b                               brif v6, block2, block5
;;
;;                                 block2:
;; @002b                               v66 = icmp.i64 ult v28, v50
;; @002b                               v71 = iadd.i64 v28, v124
;; @002b                               v72 = iadd.i64 v50, v124
;; @002b                               v74 = iadd.i32 v5, v6
;; @002b                               v26 = iconst.i64 4
;; @002b                               v97 = iconst.i32 1
;; @002b                               brif v66, block3(v28, v50, v5), block4(v71, v72, v74)
;;
;;                                 block3(v75: i64, v76: i64, v77: i32):
;; @002b                               v80 = load.i32 user2 little region0 v76
;; @002b                               store user2 little region0 v80, v75
;;                                     v131 = iconst.i64 4
;;                                     v132 = iadd v76, v131  ; v131 = 4
;; @002b                               v87 = icmp eq v132, v72
;;                                     v133 = iadd v75, v131  ; v131 = 4
;;                                     v134 = iconst.i32 1
;;                                     v135 = iadd v77, v134  ; v134 = 1
;; @002b                               brif v87, block5, block3(v133, v132, v135)
;;
;;                                 block4(v88: i64, v89: i64, v90: i32):
;;                                     v126 = iconst.i64 4
;;                                     v127 = isub v89, v126  ; v126 = 4
;; @002b                               v99 = load.i32 user2 little region0 v127
;;                                     v128 = isub v88, v126  ; v126 = 4
;; @002b                               store user2 little region0 v99, v128
;; @002b                               v100 = icmp eq v127, v50
;;                                     v129 = iconst.i32 1
;;                                     v130 = isub v90, v129  ; v129 = 1
;; @002b                               brif v100, block5, block4(v128, v127, v130)
;;
;;                                 block5:
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
