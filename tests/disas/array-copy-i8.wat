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
;; @002b                               v76 = load.i64 notrap aligned readonly can_move v0+8
;; @002b                               v8 = load.i64 notrap aligned readonly can_move v76+32
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
;; @002b                               v30 = uextend.i64 v4
;; @002b                               v32 = iadd v8, v30
;; @002b                               v34 = iadd v32, v10  ; v10 = 16
;; @002b                               v35 = load.i32 user2 readonly region0 v34
;; @002b                               v37 = uextend.i64 v5
;; @002b                               v41 = iadd v37, v15
;; @002b                               v36 = uextend.i64 v35
;; @002b                               v42 = icmp ugt v41, v36
;; @002b                               trapnz v42, user17
;; @002b                               v60 = load.i64 notrap aligned v76+40
;; @002b                               v24 = iconst.i64 20
;; @002b                               v25 = iadd v9, v24  ; v24 = 20
;; @002b                               v29 = iadd v25, v14
;; @002b                               v62 = uadd_overflow_trap v29, v15, user2
;; @002b                               v61 = iadd v8, v60
;; @002b                               v63 = icmp ugt v62, v61
;; @002b                               trapnz v63, user2
;; @002b                               v48 = iadd v32, v24  ; v24 = 20
;; @002b                               v52 = iadd v48, v37
;; @002b                               v68 = uadd_overflow_trap v52, v15, user2
;; @002b                               v69 = icmp ugt v68, v61
;; @002b                               trapnz v69, user2
;; @002b                               call fn0(v0, v29, v52, v15)
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
