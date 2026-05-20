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
;;     fn0 = colocated u805306368:3 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @002a                               trapz v2, user16
;; @002a                               v53 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v7 = load.i64 notrap aligned readonly can_move v53+32
;; @002a                               v6 = uextend.i64 v2
;; @002a                               v8 = iadd v7, v6
;; @002a                               v9 = iconst.i64 24
;; @002a                               v10 = iadd v8, v9  ; v9 = 24
;; @002a                               v11 = load.i32 user2 readonly v10
;; @002a                               v13 = uextend.i64 v3
;; @002a                               v14 = uextend.i64 v5
;; @002a                               v15 = iadd v13, v14
;; @002a                               v12 = uextend.i64 v11
;; @002a                               v16 = icmp ugt v15, v12
;; @002a                               trapnz v16, user17
;; @002a                               v19 = uload32 notrap aligned v0+56
;; @002a                               v20 = uextend.i64 v4
;; @002a                               v22 = iadd v20, v14
;; @002a                               v23 = icmp ugt v22, v19
;; @002a                               trapnz v23, heap_oob
;; @002a                               v25 = load.i64 notrap aligned v0+48
;;                                     v50 = iconst.i64 32
;; @002a                               v32 = ushr v12, v50  ; v50 = 32
;; @002a                               trapnz v32, user2
;; @002a                               v34 = iconst.i32 28
;; @002a                               v35 = uadd_overflow_trap v11, v34, user2  ; v34 = 28
;; @002a                               v39 = uadd_overflow_trap v2, v35, user2
;; @002a                               v40 = uextend.i64 v39
;; @002a                               v42 = iadd v7, v40
;;                                     v62 = iadd v3, v34  ; v34 = 28
;; @002a                               v43 = isub v35, v62
;; @002a                               v44 = uextend.i64 v43
;; @002a                               v45 = isub v42, v44
;; @002a                               v28 = iadd v25, v20
;; @002a                               call fn0(v0, v45, v28, v14)
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return
;; }
