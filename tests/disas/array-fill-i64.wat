;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut i64)))

  (func $fill-anything (param $a (ref $a)) (param $i i32) (param $v i64) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )

  (func $fill-zero (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (i64.const 0) (local.get $len))
  )

  (func $fill-minus-one (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (i64.const -1) (local.get $len))
  )

  (func $fill-bit-pattern (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (i64.const 1) (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i64, i32) tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64, v5: i32):
;; @0031                               trapz v2, user16
;; @0031                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @0031                               v7 = load.i64 notrap aligned readonly can_move v49+32
;; @0031                               v6 = uextend.i64 v2
;; @0031                               v8 = iadd v7, v6
;; @0031                               v9 = iconst.i64 16
;; @0031                               v10 = iadd v8, v9  ; v9 = 16
;; @0031                               v11 = load.i32 user2 readonly region0 v10
;; @0031                               v13 = uextend.i64 v3
;; @0031                               v14 = uextend.i64 v5
;; @0031                               v17 = iadd v13, v14
;; @0031                               v12 = uextend.i64 v11
;; @0031                               v18 = icmp ugt v17, v12
;; @0031                               trapnz v18, user17
;; @0031                               v32 = load.i64 notrap aligned v49+40
;; @0031                               v22 = iconst.i64 24
;; @0031                               v23 = iadd v8, v22  ; v22 = 24
;;                                     v53 = iconst.i64 3
;;                                     v54 = ishl v13, v53  ; v53 = 3
;; @0031                               v27 = iadd v23, v54
;;                                     v56 = ishl v14, v53  ; v53 = 3
;; @0031                               v34 = uadd_overflow_trap v27, v56, user2
;; @0031                               v33 = iadd v7, v32
;; @0031                               v35 = icmp ugt v34, v33
;; @0031                               trapnz v35, user2
;;                                     v51 = iconst.i64 0
;; @0031                               v38 = icmp eq v14, v51  ; v51 = 0
;; @0031                               v25 = iconst.i64 8
;; @0031                               v36 = iadd v27, v56
;; @0031                               brif v38, block3, block2(v27)
;;
;;                                 block2(v39: i64):
;; @0031                               store.i64 user2 little region0 v4, v39
;;                                     v58 = iconst.i64 8
;;                                     v59 = iadd v39, v58  ; v58 = 8
;; @0031                               v42 = icmp eq v59, v36
;; @0031                               brif v42, block3, block2(v59)
;;
;;                                 block3:
;; @0034                               jump block1
;;
;;                                 block1:
;; @0034                               return
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
;; @003f                               trapz v2, user16
;; @003f                               v43 = load.i64 notrap aligned readonly can_move v0+8
;; @003f                               v7 = load.i64 notrap aligned readonly can_move v43+32
;; @003f                               v6 = uextend.i64 v2
;; @003f                               v8 = iadd v7, v6
;; @003f                               v9 = iconst.i64 16
;; @003f                               v10 = iadd v8, v9  ; v9 = 16
;; @003f                               v11 = load.i32 user2 readonly region0 v10
;; @003f                               v13 = uextend.i64 v3
;; @003f                               v14 = uextend.i64 v4
;; @003f                               v17 = iadd v13, v14
;; @003f                               v12 = uextend.i64 v11
;; @003f                               v18 = icmp ugt v17, v12
;; @003f                               trapnz v18, user17
;; @003f                               v32 = load.i64 notrap aligned v43+40
;; @003f                               v22 = iconst.i64 24
;; @003f                               v23 = iadd v8, v22  ; v22 = 24
;;                                     v46 = iconst.i64 3
;;                                     v47 = ishl v13, v46  ; v46 = 3
;; @003f                               v27 = iadd v23, v47
;;                                     v49 = ishl v14, v46  ; v46 = 3
;; @003f                               v34 = uadd_overflow_trap v27, v49, user2
;; @003f                               v33 = iadd v7, v32
;; @003f                               v35 = icmp ugt v34, v33
;; @003f                               trapnz v35, user2
;; @003f                               v36 = iconst.i32 0
;; @003f                               call fn0(v0, v27, v36, v49)  ; v36 = 0
;; @0042                               jump block1
;;
;;                                 block1:
;; @0042                               return
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
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @004d                               trapz v2, user16
;; @004d                               v43 = load.i64 notrap aligned readonly can_move v0+8
;; @004d                               v7 = load.i64 notrap aligned readonly can_move v43+32
;; @004d                               v6 = uextend.i64 v2
;; @004d                               v8 = iadd v7, v6
;; @004d                               v9 = iconst.i64 16
;; @004d                               v10 = iadd v8, v9  ; v9 = 16
;; @004d                               v11 = load.i32 user2 readonly region0 v10
;; @004d                               v13 = uextend.i64 v3
;; @004d                               v14 = uextend.i64 v4
;; @004d                               v17 = iadd v13, v14
;; @004d                               v12 = uextend.i64 v11
;; @004d                               v18 = icmp ugt v17, v12
;; @004d                               trapnz v18, user17
;; @004d                               v32 = load.i64 notrap aligned v43+40
;; @004d                               v22 = iconst.i64 24
;; @004d                               v23 = iadd v8, v22  ; v22 = 24
;;                                     v47 = iconst.i64 3
;;                                     v48 = ishl v13, v47  ; v47 = 3
;; @004d                               v27 = iadd v23, v48
;;                                     v50 = ishl v14, v47  ; v47 = 3
;; @004d                               v34 = uadd_overflow_trap v27, v50, user2
;; @004d                               v33 = iadd v7, v32
;; @004d                               v35 = icmp ugt v34, v33
;; @004d                               trapnz v35, user2
;; @004d                               v36 = iconst.i32 255
;; @004d                               call fn0(v0, v27, v36, v50)  ; v36 = 255
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
;;
;; function u0:3(i64 vmctx, i64, i32, i32, i32) tail {
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
;; @005b                               trapz v2, user16
;; @005b                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @005b                               v7 = load.i64 notrap aligned readonly can_move v49+32
;; @005b                               v6 = uextend.i64 v2
;; @005b                               v8 = iadd v7, v6
;; @005b                               v9 = iconst.i64 16
;; @005b                               v10 = iadd v8, v9  ; v9 = 16
;; @005b                               v11 = load.i32 user2 readonly region0 v10
;; @005b                               v13 = uextend.i64 v3
;; @005b                               v14 = uextend.i64 v4
;; @005b                               v17 = iadd v13, v14
;; @005b                               v12 = uextend.i64 v11
;; @005b                               v18 = icmp ugt v17, v12
;; @005b                               trapnz v18, user17
;; @005b                               v32 = load.i64 notrap aligned v49+40
;; @005b                               v22 = iconst.i64 24
;; @005b                               v23 = iadd v8, v22  ; v22 = 24
;;                                     v53 = iconst.i64 3
;;                                     v54 = ishl v13, v53  ; v53 = 3
;; @005b                               v27 = iadd v23, v54
;;                                     v56 = ishl v14, v53  ; v53 = 3
;; @005b                               v34 = uadd_overflow_trap v27, v56, user2
;; @005b                               v33 = iadd v7, v32
;; @005b                               v35 = icmp ugt v34, v33
;; @005b                               trapnz v35, user2
;;                                     v51 = iconst.i64 0
;; @005b                               v38 = icmp eq v14, v51  ; v51 = 0
;; @0057                               v5 = iconst.i64 1
;; @005b                               v25 = iconst.i64 8
;; @005b                               v36 = iadd v27, v56
;; @005b                               brif v38, block3, block2(v27)
;;
;;                                 block2(v39: i64):
;;                                     v58 = iconst.i64 1
;; @005b                               store user2 little region0 v58, v39  ; v58 = 1
;;                                     v59 = iconst.i64 8
;;                                     v60 = iadd v39, v59  ; v59 = 8
;; @005b                               v42 = icmp eq v60, v36
;; @005b                               brif v42, block3, block2(v60)
;;
;;                                 block3:
;; @005e                               jump block1
;;
;;                                 block1:
;; @005e                               return
;; }
