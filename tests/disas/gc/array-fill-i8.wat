;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut i8)))

  (func $fill (param $a (ref $a)) (param $i i32) (param $v i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:5 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0027                               trapz v2, user16
;; @0027                               v48 = load.i64 notrap aligned readonly can_move v0+8
;; @0027                               v7 = load.i64 notrap aligned readonly can_move v48+32
;; @0027                               v6 = uextend.i64 v2
;; @0027                               v8 = iadd v7, v6
;; @0027                               v9 = iconst.i64 24
;; @0027                               v10 = iadd v8, v9  ; v9 = 24
;; @0027                               v11 = load.i32 user2 readonly v10
;; @0027                               v12 = uadd_overflow_trap v3, v5, user17
;; @0027                               v13 = icmp ugt v12, v11
;; @0027                               trapnz v13, user17
;; @0027                               v15 = uextend.i64 v11
;;                                     v47 = iconst.i64 32
;; @0027                               v17 = ushr v15, v47  ; v47 = 32
;; @0027                               trapnz v17, user2
;; @0027                               v19 = iconst.i32 28
;; @0027                               v20 = uadd_overflow_trap v11, v19, user2  ; v19 = 28
;; @0027                               v24 = uadd_overflow_trap v2, v20, user2
;; @0027                               v36 = load.i64 notrap aligned v48+40
;; @0027                               v25 = uextend.i64 v24
;; @0027                               v27 = iadd v7, v25
;;                                     v56 = iadd v3, v19  ; v19 = 28
;; @0027                               v28 = isub v20, v56
;; @0027                               v29 = uextend.i64 v28
;; @0027                               v30 = isub v27, v29
;; @0027                               v32 = uextend.i64 v5
;; @0027                               v33 = iadd v30, v32
;; @0027                               v37 = iadd v7, v36
;; @0027                               v38 = icmp ugt v33, v37
;; @0027                               trapnz v38, user2
;; @0027                               call fn0(v0, v30, v4, v32)
;; @002a                               jump block1
;;
;;                                 block1:
;; @002a                               return
;; }
