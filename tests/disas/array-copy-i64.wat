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
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:1 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32):
;; @002b                               trapz v2, user16
;; @002b                               v81 = load.i64 notrap aligned readonly can_move v0+8
;; @002b                               v8 = load.i64 notrap aligned readonly can_move v81+32
;; @002b                               v7 = uextend.i64 v2
;; @002b                               v9 = iadd v8, v7
;; @002b                               v10 = iconst.i64 16
;; @002b                               v11 = iadd v9, v10  ; v10 = 16
;; @002b                               v12 = load.i32 user2 readonly region0 v11
;; @002b                               v14 = uextend.i64 v3
;; @002b                               v15 = uextend.i64 v6
;; @002b                               v17 = iadd v14, v15
;; @002b                               v13 = uextend.i64 v12
;; @002b                               v18 = icmp ugt v17, v13
;; @002b                               trapnz v18, user17
;; @002b                               trapz v4, user16
;; @002b                               v27 = uextend.i64 v4
;; @002b                               v29 = iadd v8, v27
;; @002b                               v31 = iadd v29, v10  ; v10 = 16
;; @002b                               v32 = load.i32 user2 readonly region0 v31
;; @002b                               v34 = uextend.i64 v5
;; @002b                               v37 = iadd v34, v15
;; @002b                               v33 = uextend.i64 v32
;; @002b                               v38 = icmp ugt v37, v33
;; @002b                               trapnz v38, user17
;; @002b                               v51 = load.i64 notrap aligned v81+40
;; @002b                               v22 = iconst.i64 24
;; @002b                               v23 = iadd v9, v22  ; v22 = 24
;;                                     v85 = iconst.i64 3
;;                                     v86 = ishl v14, v85  ; v85 = 3
;; @002b                               v26 = iadd v23, v86
;;                                     v90 = ishl v15, v85  ; v85 = 3
;; @002b                               v53 = uadd_overflow_trap v26, v90, user2
;; @002b                               v52 = iadd v8, v51
;; @002b                               v54 = icmp ugt v53, v52
;; @002b                               trapnz v54, user2
;; @002b                               v43 = iadd v29, v22  ; v22 = 24
;;                                     v88 = ishl v34, v85  ; v85 = 3
;; @002b                               v46 = iadd v43, v88
;; @002b                               v58 = uadd_overflow_trap v46, v90, user2
;; @002b                               v59 = icmp ugt v58, v52
;; @002b                               trapnz v59, user2
;; @002b                               call fn0(v0, v26, v46, v90)
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
