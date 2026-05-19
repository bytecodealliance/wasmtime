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
;;     fn0 = colocated u805306368:4 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @002a                               trapz v2, user16
;; @002a                               v50 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v7 = load.i64 notrap aligned readonly can_move v50+32
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
;; @002a                               v24 = load.i64 notrap aligned v0+48
;;                                     v48 = iconst.i64 32
;; @002a                               v30 = ushr v12, v48  ; v48 = 32
;; @002a                               trapnz v30, user2
;; @002a                               v32 = iconst.i32 28
;; @002a                               v33 = uadd_overflow_trap v11, v32, user2  ; v32 = 28
;; @002a                               v37 = uadd_overflow_trap v2, v33, user2
;; @002a                               v38 = uextend.i64 v37
;; @002a                               v40 = iadd v7, v38
;;                                     v58 = iadd v3, v32  ; v32 = 28
;; @002a                               v41 = isub v33, v58
;; @002a                               v42 = uextend.i64 v41
;; @002a                               v43 = isub v40, v42
;; @002a                               v26 = iadd v24, v20
;; @002a                               call fn0(v0, v43, v26, v14)
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return
;; }
