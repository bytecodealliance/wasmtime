;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut i64)))

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
;;     sig0 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:4 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32):
;; @002b                               trapz v4, user16
;; @002b                               v80 = load.i64 notrap aligned readonly can_move v0+8
;; @002b                               v11 = load.i64 notrap aligned readonly can_move v80+32
;; @002b                               v10 = uextend.i64 v4
;; @002b                               v12 = iadd v11, v10
;; @002b                               v13 = iconst.i64 24
;; @002b                               v14 = iadd v12, v13  ; v13 = 24
;; @002b                               v15 = load.i32 user2 readonly v14
;; @002b                               v16 = uadd_overflow_trap v5, v6, user17
;; @002b                               v17 = icmp ugt v16, v15
;; @002b                               trapnz v17, user17
;; @002b                               v19 = uextend.i64 v15
;;                                     v85 = iconst.i64 3
;;                                     v86 = ishl v19, v85  ; v85 = 3
;;                                     v79 = iconst.i64 32
;; @002b                               v21 = ushr v86, v79  ; v79 = 32
;; @002b                               trapnz v21, user2
;;                                     v82 = iconst.i32 3
;;                                     v94 = ishl v15, v82  ; v82 = 3
;; @002b                               v23 = iconst.i32 32
;; @002b                               v24 = uadd_overflow_trap v94, v23, user2  ; v23 = 32
;; @002b                               v28 = uadd_overflow_trap v4, v24, user2
;; @002b                               trapz v2, user16
;; @002b                               v36 = uextend.i64 v2
;; @002b                               v38 = iadd v11, v36
;; @002b                               v40 = iadd v38, v13  ; v13 = 24
;; @002b                               v41 = load.i32 user2 readonly v40
;; @002b                               v42 = uadd_overflow_trap v3, v6, user17
;; @002b                               v43 = icmp ugt v42, v41
;; @002b                               trapnz v43, user17
;; @002b                               v45 = uextend.i64 v41
;;                                     v104 = ishl v45, v85  ; v85 = 3
;; @002b                               v47 = ushr v104, v79  ; v79 = 32
;; @002b                               trapnz v47, user2
;;                                     v111 = ishl v41, v82  ; v82 = 3
;; @002b                               v50 = uadd_overflow_trap v111, v23, user2  ; v23 = 32
;; @002b                               v54 = uadd_overflow_trap v2, v50, user2
;; @002b                               v63 = load.i64 notrap aligned v80+40
;; @002b                               v29 = uextend.i64 v28
;; @002b                               v31 = iadd v11, v29
;;                                     v100 = ishl v5, v82  ; v82 = 3
;;                                     v102 = iadd v100, v23  ; v23 = 32
;; @002b                               v32 = isub v24, v102
;; @002b                               v33 = uextend.i64 v32
;; @002b                               v34 = isub v31, v33
;;                                     v83 = ishl v6, v82  ; v82 = 3
;; @002b                               v9 = uextend.i64 v83
;; @002b                               v35 = iadd v34, v9
;; @002b                               v64 = iadd v11, v63
;; @002b                               v65 = icmp ugt v35, v64
;; @002b                               trapnz v65, user2
;; @002b                               v55 = uextend.i64 v54
;; @002b                               v57 = iadd v11, v55
;;                                     v117 = ishl v3, v82  ; v82 = 3
;;                                     v119 = iadd v117, v23  ; v23 = 32
;; @002b                               v58 = isub v50, v119
;; @002b                               v59 = uextend.i64 v58
;; @002b                               v60 = isub v57, v59
;; @002b                               v61 = iadd v60, v9
;; @002b                               v66 = icmp ugt v61, v64
;; @002b                               trapnz v66, user2
;; @002b                               call fn0(v0, v60, v34, v9)
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
