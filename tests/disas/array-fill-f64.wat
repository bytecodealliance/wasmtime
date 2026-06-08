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
;; @0030                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @0030                               v7 = load.i64 notrap aligned readonly can_move v49+32
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
;; @0030                               v32 = load.i64 notrap aligned v49+40
;; @0030                               v22 = iconst.i64 24
;; @0030                               v23 = iadd v8, v22  ; v22 = 24
;;                                     v53 = iconst.i64 3
;;                                     v54 = ishl v13, v53  ; v53 = 3
;; @0030                               v27 = iadd v23, v54
;;                                     v56 = ishl v14, v53  ; v53 = 3
;; @0030                               v34 = uadd_overflow_trap v27, v56, user2
;; @0030                               v33 = iadd v7, v32
;; @0030                               v35 = icmp ugt v34, v33
;; @0030                               trapnz v35, user2
;;                                     v51 = iconst.i64 0
;; @0030                               v38 = icmp eq v14, v51  ; v51 = 0
;; @0030                               v25 = iconst.i64 8
;; @0030                               v36 = iadd v27, v56
;; @0030                               brif v38, block3, block2(v27)
;;
;;                                 block2(v39: i64):
;; @0030                               store.f64 user2 little region0 v4, v39
;;                                     v58 = iconst.i64 8
;;                                     v59 = iadd v39, v58  ; v58 = 8
;; @0030                               v42 = icmp eq v59, v36
;; @0030                               brif v42, block3, block2(v59)
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
;; @0045                               v43 = load.i64 notrap aligned readonly can_move v0+8
;; @0045                               v7 = load.i64 notrap aligned readonly can_move v43+32
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
;; @0045                               v32 = load.i64 notrap aligned v43+40
;; @0045                               v22 = iconst.i64 24
;; @0045                               v23 = iadd v8, v22  ; v22 = 24
;;                                     v47 = iconst.i64 3
;;                                     v48 = ishl v13, v47  ; v47 = 3
;; @0045                               v27 = iadd v23, v48
;;                                     v50 = ishl v14, v47  ; v47 = 3
;; @0045                               v34 = uadd_overflow_trap v27, v50, user2
;; @0045                               v33 = iadd v7, v32
;; @0045                               v35 = icmp ugt v34, v33
;; @0045                               trapnz v35, user2
;; @0045                               v36 = iconst.i32 0
;; @0045                               call fn0(v0, v27, v36, v50)  ; v36 = 0
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
;; @005a                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @005a                               v7 = load.i64 notrap aligned readonly can_move v49+32
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
;; @005a                               v32 = load.i64 notrap aligned v49+40
;; @005a                               v22 = iconst.i64 24
;; @005a                               v23 = iadd v8, v22  ; v22 = 24
;;                                     v53 = iconst.i64 3
;;                                     v54 = ishl v13, v53  ; v53 = 3
;; @005a                               v27 = iadd v23, v54
;;                                     v56 = ishl v14, v53  ; v53 = 3
;; @005a                               v34 = uadd_overflow_trap v27, v56, user2
;; @005a                               v33 = iadd v7, v32
;; @005a                               v35 = icmp ugt v34, v33
;; @005a                               trapnz v35, user2
;;                                     v51 = iconst.i64 0
;; @005a                               v38 = icmp eq v14, v51  ; v51 = 0
;; @004f                               v5 = f64const 0x1.0000000000000p0
;; @005a                               v25 = iconst.i64 8
;; @005a                               v36 = iadd v27, v56
;; @005a                               brif v38, block3, block2(v27)
;;
;;                                 block2(v39: i64):
;;                                     v58 = f64const 0x1.0000000000000p0
;; @005a                               store user2 little region0 v58, v39  ; v58 = 0x1.0000000000000p0
;;                                     v59 = iconst.i64 8
;;                                     v60 = iadd v39, v59  ; v59 = 8
;; @005a                               v42 = icmp eq v60, v36
;; @005a                               brif v42, block3, block2(v60)
;;
;;                                 block3:
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return
;; }
