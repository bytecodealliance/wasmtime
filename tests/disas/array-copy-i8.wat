;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut i8)))

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
;; @002b                               v79 = load.i64 notrap aligned readonly can_move v0+8
;; @002b                               v8 = load.i64 notrap aligned readonly can_move v79+32
;; @002b                               v7 = uextend.i64 v4
;; @002b                               v9 = iadd v8, v7
;; @002b                               v10 = iconst.i64 24
;; @002b                               v11 = iadd v9, v10  ; v10 = 24
;; @002b                               v12 = load.i32 user2 readonly v11
;; @002b                               v13 = uadd_overflow_trap v5, v6, user17
;; @002b                               v14 = icmp ugt v13, v12
;; @002b                               trapnz v14, user17
;; @002b                               v16 = uextend.i64 v12
;;                                     v78 = iconst.i64 32
;; @002b                               v18 = ushr v16, v78  ; v78 = 32
;; @002b                               trapnz v18, user2
;; @002b                               v20 = iconst.i32 28
;; @002b                               v21 = uadd_overflow_trap v12, v20, user2  ; v20 = 28
;; @002b                               v25 = uadd_overflow_trap v4, v21, user2
;; @002b                               trapz v2, user16
;; @002b                               v32 = uextend.i64 v2
;; @002b                               v34 = iadd v8, v32
;; @002b                               v36 = iadd v34, v10  ; v10 = 24
;; @002b                               v37 = load.i32 user2 readonly v36
;; @002b                               v38 = uadd_overflow_trap v3, v6, user17
;; @002b                               v39 = icmp ugt v38, v37
;; @002b                               trapnz v39, user17
;; @002b                               v41 = uextend.i64 v37
;; @002b                               v43 = ushr v41, v78  ; v78 = 32
;; @002b                               trapnz v43, user2
;; @002b                               v46 = uadd_overflow_trap v37, v20, user2  ; v20 = 28
;; @002b                               v50 = uadd_overflow_trap v2, v46, user2
;; @002b                               v62 = load.i64 notrap aligned v79+40
;; @002b                               v26 = uextend.i64 v25
;; @002b                               v28 = iadd v8, v26
;;                                     v87 = iadd v5, v20  ; v20 = 28
;; @002b                               v29 = isub v21, v87
;; @002b                               v30 = uextend.i64 v29
;; @002b                               v31 = isub v28, v30
;; @002b                               v58 = uextend.i64 v6
;; @002b                               v59 = iadd v31, v58
;; @002b                               v63 = iadd v8, v62
;; @002b                               v64 = icmp ugt v59, v63
;; @002b                               trapnz v64, user2
;; @002b                               v51 = uextend.i64 v50
;; @002b                               v53 = iadd v8, v51
;;                                     v92 = iadd v3, v20  ; v20 = 28
;; @002b                               v54 = isub v46, v92
;; @002b                               v55 = uextend.i64 v54
;; @002b                               v56 = isub v53, v55
;; @002b                               v60 = iadd v56, v58
;; @002b                               v65 = icmp ugt v60, v63
;; @002b                               trapnz v65, user2
;; @002b                               call fn0(v0, v56, v31, v58)
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
