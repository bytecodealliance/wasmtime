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
;; @002a                               v57 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v7 = load.i64 notrap aligned readonly can_move v57+32
;; @002a                               v6 = uextend.i64 v2
;; @002a                               v8 = iadd v7, v6
;; @002a                               v9 = iconst.i64 24
;; @002a                               v10 = iadd v8, v9  ; v9 = 24
;; @002a                               v11 = load.i32 user2 readonly v10
;; @002a                               v13 = uextend.i64 v3
;; @002a                               v14 = uextend.i64 v5
;; @002a                               v16 = iadd v13, v14
;; @002a                               v12 = uextend.i64 v11
;; @002a                               v17 = icmp ugt v16, v12
;; @002a                               trapnz v17, user17
;; @002a                               v26 = uload32 notrap aligned v0+56
;; @002a                               v27 = uextend.i64 v4
;; @002a                               v30 = iadd v27, v14
;; @002a                               v31 = icmp ugt v30, v26
;; @002a                               trapnz v31, heap_oob
;; @002a                               v33 = load.i64 notrap aligned v0+48
;; @002a                               v40 = load.i64 notrap aligned v57+40
;;                                     v53 = iconst.i64 28
;; @002a                               v21 = iadd v8, v53  ; v53 = 28
;; @002a                               v24 = iadd v21, v13
;; @002a                               v42 = uadd_overflow_trap v24, v14, user2
;; @002a                               v41 = iadd v7, v40
;; @002a                               v43 = icmp ugt v42, v41
;; @002a                               trapnz v43, user2
;; @002a                               v35 = iadd v33, v27
;; @002a                               call fn0(v0, v24, v35, v14)
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return
;; }
