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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32):
;; @002b                               trapz v2, user16
;; @002b                               v109 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002b                               v8 = load.i64 notrap aligned readonly can_move v109+32
;; @002b                               v7 = uextend.i64 v2
;; @002b                               v9 = iadd v8, v7
;; @002b                               v10 = iconst.i64 16
;; @002b                               v11 = iadd v9, v10  ; v10 = 16
;; @002b                               v12 = load.i32 user2 readonly region1 v11
;; @002b                               v14 = uextend.i64 v3
;; @002b                               v15 = uextend.i64 v6
;; @002b                               v18 = iadd v14, v15
;; @002b                               v13 = uextend.i64 v12
;; @002b                               v19 = icmp ugt v18, v13
;; @002b                               trapnz v19, user17
;; @002b                               trapz v4, user16
;; @002b                               v30 = uextend.i64 v4
;; @002b                               v32 = iadd v8, v30
;; @002b                               v34 = iadd v32, v10  ; v10 = 16
;; @002b                               v35 = load.i32 user2 readonly region1 v34
;; @002b                               v37 = uextend.i64 v5
;; @002b                               v41 = iadd v37, v15
;; @002b                               v36 = uextend.i64 v35
;; @002b                               v42 = icmp ugt v41, v36
;; @002b                               trapnz v42, user17
;; @002b                               v61 = load.i64 notrap aligned v109+40
;; @002b                               v24 = iconst.i64 20
;; @002b                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v113 = iconst.i64 2
;;                                     v114 = ishl v14, v113  ; v113 = 2
;; @002b                               v29 = iadd v25, v114
;;                                     v118 = ishl v15, v113  ; v113 = 2
;; @002b                               v63 = uadd_overflow_trap v29, v118, user2
;; @002b                               v62 = iadd v8, v61
;; @002b                               v64 = icmp ugt v63, v62
;; @002b                               trapnz v64, user2
;; @002b                               v48 = iadd v32, v24  ; v24 = 20
;;                                     v116 = ishl v37, v113  ; v113 = 2
;; @002b                               v52 = iadd v48, v116
;; @002b                               v70 = uadd_overflow_trap v52, v118, user2
;; @002b                               v71 = icmp ugt v70, v62
;; @002b                               trapnz v71, user2
;; @002b                               brif v6, block2, block5
;;
;;                                 block2:
;; @002b                               v72 = icmp.i64 ult v29, v52
;; @002b                               v77 = iadd.i64 v29, v118
;; @002b                               v78 = iadd.i64 v52, v118
;; @002b                               v80 = iadd.i32 v5, v6
;; @002b                               v27 = iconst.i64 4
;; @002b                               v103 = iconst.i32 1
;; @002b                               brif v72, block3(v29, v52, v5), block4(v77, v78, v80)
;;
;;                                 block3(v81: i64, v82: i64, v83: i32):
;; @002b                               v86 = load.i32 user2 little region1 v82
;; @002b                               store user2 little region1 v86, v81
;;                                     v125 = iconst.i64 4
;;                                     v126 = iadd v82, v125  ; v125 = 4
;; @002b                               v93 = icmp eq v126, v78
;;                                     v127 = iadd v81, v125  ; v125 = 4
;;                                     v128 = iconst.i32 1
;;                                     v129 = iadd v83, v128  ; v128 = 1
;; @002b                               brif v93, block5, block3(v127, v126, v129)
;;
;;                                 block4(v94: i64, v95: i64, v96: i32):
;;                                     v120 = iconst.i64 4
;;                                     v121 = isub v95, v120  ; v120 = 4
;; @002b                               v105 = load.i32 user2 little region1 v121
;;                                     v122 = isub v94, v120  ; v120 = 4
;; @002b                               store user2 little region1 v105, v122
;; @002b                               v106 = icmp eq v121, v52
;;                                     v123 = iconst.i32 1
;;                                     v124 = isub v96, v123  ; v123 = 1
;; @002b                               brif v106, block5, block4(v122, v121, v124)
;;
;;                                 block5:
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
