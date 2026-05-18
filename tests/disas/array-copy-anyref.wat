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
;; @002b                               trapz v2, user16
;; @002b                               v102 = load.i64 notrap aligned readonly can_move v0+8
;; @002b                               v8 = load.i64 notrap aligned readonly can_move v102+32
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
;; @002b                               v49 = load.i64 notrap aligned v102+40
;;                                     v98 = iconst.i64 20
;; @002b                               v22 = iadd v9, v98  ; v98 = 20
;;                                     v106 = iconst.i64 2
;;                                     v107 = ishl v14, v106  ; v106 = 2
;; @002b                               v25 = iadd v22, v107
;;                                     v111 = ishl v15, v106  ; v106 = 2
;; @002b                               v51 = uadd_overflow_trap v25, v111, user2
;; @002b                               v50 = iadd v8, v49
;; @002b                               v52 = icmp ugt v51, v50
;; @002b                               trapnz v52, user2
;; @002b                               brif v15, block2, block5
;;
;;                                 block2:
;;                                     v116 = iconst.i64 20
;;                                     v117 = iadd.i64 v28, v116  ; v116 = 20
;;                                     v118 = iconst.i64 2
;;                                     v119 = ishl.i64 v33, v118  ; v118 = 2
;; @002b                               v44 = iadd v117, v119
;; @002b                               v58 = icmp.i64 ult v25, v44
;; @002b                               v60 = iadd.i64 v25, v111
;; @002b                               v61 = iadd v44, v111
;; @002b                               v63 = iadd.i32 v5, v6
;;                                     v97 = iconst.i64 4
;; @002b                               v77 = iconst.i32 1
;; @002b                               brif v58, block3(v25, v44, v5), block4(v60, v61, v63)
;;
;;                                 block3(v64: i64, v65: i64, v66: i32):
;; @002b                               v67 = load.i32 user2 little v65
;; @002b                               store user2 little v67, v64
;;                                     v125 = iconst.i64 4
;;                                     v126 = iadd v65, v125  ; v125 = 4
;; @002b                               v71 = icmp eq v126, v61
;;                                     v127 = iadd v64, v125  ; v125 = 4
;;                                     v128 = iconst.i32 1
;;                                     v129 = iadd v66, v128  ; v128 = 1
;; @002b                               brif v71, block5, block3(v127, v126, v129)
;;
;;                                 block4(v72: i64, v73: i64, v74: i32):
;;                                     v120 = iconst.i64 4
;;                                     v121 = isub v73, v120  ; v120 = 4
;; @002b                               v79 = load.i32 user2 little v121
;;                                     v122 = isub v72, v120  ; v120 = 4
;; @002b                               store user2 little v79, v122
;; @002b                               v80 = icmp eq v121, v44
;;                                     v123 = iconst.i32 1
;;                                     v124 = isub v74, v123  ; v123 = 1
;; @002b                               brif v80, block5, block4(v122, v121, v124)
;;
;;                                 block5:
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
