;;! target = "x86_64"
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (data $passive "this is a passive data segment")
  (type $a (array (mut i8)))

  (func $a (param (ref $a) i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    array.init_data $a $passive)
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32) tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @002a                               trapz v2, user16
;; @002a                               v58 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v7 = load.i64 notrap aligned readonly can_move v58+32
;; @002a                               v6 = uextend.i64 v2
;; @002a                               v8 = iadd v7, v6
;; @002a                               v9 = iconst.i64 16
;; @002a                               v10 = iadd v8, v9  ; v9 = 16
;; @002a                               v11 = load.i32 user2 readonly region0 v10
;; @002a                               v13 = uextend.i64 v3
;; @002a                               v14 = uextend.i64 v5
;; @002a                               v16 = iadd v13, v14
;; @002a                               v12 = uextend.i64 v11
;; @002a                               v17 = icmp ugt v16, v12
;; @002a                               trapnz v17, user17
;; @002a                               v27 = load.i32 notrap aligned v0+56
;; @002a                               v29 = uextend.i64 v4
;; @002a                               v32 = iadd v29, v14
;; @002a                               v28 = uextend.i64 v27
;; @002a                               v33 = icmp ugt v32, v28
;; @002a                               trapnz v33, heap_oob
;; @002a                               v35 = load.i64 notrap aligned v0+48
;; @002a                               v42 = load.i64 notrap aligned v58+40
;; @002a                               v21 = iconst.i64 20
;; @002a                               v22 = iadd v8, v21  ; v21 = 20
;; @002a                               v25 = iadd v22, v13
;; @002a                               v44 = uadd_overflow_trap v25, v14, user2
;; @002a                               v43 = iadd v7, v42
;; @002a                               v45 = icmp ugt v44, v43
;; @002a                               trapnz v45, user2
;; @002a                               v37 = iadd v35, v29
;; @002a                               call fn0(v0, v25, v37, v14)
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return
;; }
