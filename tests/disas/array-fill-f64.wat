;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut f64)))

  (func $fill-anything (param $a (ref $a)) (param $i i32) (param $v f64) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )

  (func $fill-zero (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (f64.const 0) (local.get $len))
  )

  (func $fill-bit-pattern (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (f64.const 1) (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, f64, i32) tail {
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: f64, v5: i32):
;; @0030                               trapz v2, user16
;; @0030                               v47 = load.i64 notrap aligned readonly can_move v0+8
;; @0030                               v7 = load.i64 notrap aligned readonly can_move v47+32
;; @0030                               v6 = uextend.i64 v2
;; @0030                               v8 = iadd v7, v6
;; @0030                               v9 = iconst.i64 16
;; @0030                               v10 = iadd v8, v9  ; v9 = 16
;; @0030                               v11 = load.i32 user2 readonly region0 v10
;; @0030                               v13 = uextend.i64 v3
;; @0030                               v14 = uextend.i64 v5
;; @0030                               v17 = iadd v13, v14
;; @0030                               v12 = uextend.i64 v11
;; @0030                               v18 = icmp ugt v17, v12
;; @0030                               trapnz v18, user17
;; @0030                               v34 = load.i64 notrap aligned v47+40
;; @0030                               v23 = iconst.i64 24
;; @0030                               v24 = iadd v8, v23  ; v23 = 24
;;                                     v51 = iconst.i64 3
;;                                     v52 = ishl v13, v51  ; v51 = 3
;; @0030                               v28 = iadd v24, v52
;;                                     v54 = ishl v14, v51  ; v51 = 3
;; @0030                               v36 = uadd_overflow_trap v28, v54, user2
;; @0030                               v35 = iadd v7, v34
;; @0030                               v37 = icmp ugt v36, v35
;; @0030                               trapnz v37, user2
;;                                     v49 = iconst.i64 0
;; @0030                               v40 = icmp eq v14, v49  ; v49 = 0
;; @0030                               v26 = iconst.i64 8
;; @0030                               v38 = iadd v28, v54
;; @0030                               brif v40, block3, block2(v28)
;;
;;                                 block2(v41: i64):
;; @0030                               store.f64 user2 little region0 v4, v41
;;                                     v56 = iconst.i64 8
;;                                     v57 = iadd v41, v56  ; v56 = 8
;; @0030                               v44 = icmp eq v57, v38
;; @0030                               brif v44, block3, block2(v57)
;;
;;                                 block3:
;; @0033                               jump block1
;;
;;                                 block1:
;; @0033                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0045                               trapz v2, user16
;; @0045                               v41 = load.i64 notrap aligned readonly can_move v0+8
;; @0045                               v7 = load.i64 notrap aligned readonly can_move v41+32
;; @0045                               v6 = uextend.i64 v2
;; @0045                               v8 = iadd v7, v6
;; @0045                               v9 = iconst.i64 16
;; @0045                               v10 = iadd v8, v9  ; v9 = 16
;; @0045                               v11 = load.i32 user2 readonly region0 v10
;; @0045                               v13 = uextend.i64 v3
;; @0045                               v14 = uextend.i64 v4
;; @0045                               v17 = iadd v13, v14
;; @0045                               v12 = uextend.i64 v11
;; @0045                               v18 = icmp ugt v17, v12
;; @0045                               trapnz v18, user17
;; @0045                               v34 = load.i64 notrap aligned v41+40
;; @0045                               v23 = iconst.i64 24
;; @0045                               v24 = iadd v8, v23  ; v23 = 24
;;                                     v45 = iconst.i64 3
;;                                     v46 = ishl v13, v45  ; v45 = 3
;; @0045                               v28 = iadd v24, v46
;;                                     v48 = ishl v14, v45  ; v45 = 3
;; @0045                               v36 = uadd_overflow_trap v28, v48, user2
;; @0045                               v35 = iadd v7, v34
;; @0045                               v37 = icmp ugt v36, v35
;; @0045                               trapnz v37, user2
;; @0045                               v38 = iconst.i32 0
;; @0045                               call fn0(v0, v28, v38, v48)  ; v38 = 0
;; @0048                               jump block1
;;
;;                                 block1:
;; @0048                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @005a                               trapz v2, user16
;; @005a                               v47 = load.i64 notrap aligned readonly can_move v0+8
;; @005a                               v7 = load.i64 notrap aligned readonly can_move v47+32
;; @005a                               v6 = uextend.i64 v2
;; @005a                               v8 = iadd v7, v6
;; @005a                               v9 = iconst.i64 16
;; @005a                               v10 = iadd v8, v9  ; v9 = 16
;; @005a                               v11 = load.i32 user2 readonly region0 v10
;; @005a                               v13 = uextend.i64 v3
;; @005a                               v14 = uextend.i64 v4
;; @005a                               v17 = iadd v13, v14
;; @005a                               v12 = uextend.i64 v11
;; @005a                               v18 = icmp ugt v17, v12
;; @005a                               trapnz v18, user17
;; @005a                               v34 = load.i64 notrap aligned v47+40
;; @005a                               v23 = iconst.i64 24
;; @005a                               v24 = iadd v8, v23  ; v23 = 24
;;                                     v51 = iconst.i64 3
;;                                     v52 = ishl v13, v51  ; v51 = 3
;; @005a                               v28 = iadd v24, v52
;;                                     v54 = ishl v14, v51  ; v51 = 3
;; @005a                               v36 = uadd_overflow_trap v28, v54, user2
;; @005a                               v35 = iadd v7, v34
;; @005a                               v37 = icmp ugt v36, v35
;; @005a                               trapnz v37, user2
;;                                     v49 = iconst.i64 0
;; @005a                               v40 = icmp eq v14, v49  ; v49 = 0
;; @004f                               v5 = f64const 0x1.0000000000000p0
;; @005a                               v26 = iconst.i64 8
;; @005a                               v38 = iadd v28, v54
;; @005a                               brif v40, block3, block2(v28)
;;
;;                                 block2(v41: i64):
;;                                     v56 = f64const 0x1.0000000000000p0
;; @005a                               store user2 little region0 v56, v41  ; v56 = 0x1.0000000000000p0
;;                                     v57 = iconst.i64 8
;;                                     v58 = iadd v41, v57  ; v57 = 8
;; @005a                               v44 = icmp eq v58, v38
;; @005a                               brif v44, block3, block2(v58)
;;
;;                                 block3:
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return
;; }
