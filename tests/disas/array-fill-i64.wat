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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64, v5: i32):
;; @0031                               trapz v2, user16
;; @0031                               v46 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0031                               v7 = load.i64 notrap aligned readonly can_move v46+32
;; @0031                               v6 = uextend.i64 v2
;; @0031                               v8 = iadd v7, v6
;; @0031                               v9 = iconst.i64 16
;; @0031                               v10 = iadd v8, v9  ; v9 = 16
;; @0031                               v11 = load.i32 user2 readonly region1 v10
;; @0031                               v13 = uextend.i64 v3
;; @0031                               v14 = uextend.i64 v5
;; @0031                               v17 = iadd v13, v14
;; @0031                               v12 = uextend.i64 v11
;; @0031                               v18 = icmp ugt v17, v12
;; @0031                               trapnz v18, user17
;; @0031                               v35 = load.i64 notrap aligned v46+40
;; @0031                               v23 = iconst.i64 24
;; @0031                               v24 = iadd v8, v23  ; v23 = 24
;;                                     v50 = iconst.i64 3
;;                                     v51 = ishl v13, v50  ; v50 = 3
;; @0031                               v28 = iadd v24, v51
;;                                     v53 = ishl v14, v50  ; v50 = 3
;; @0031                               v37 = uadd_overflow_trap v28, v53, user2
;; @0031                               v36 = iadd v7, v35
;; @0031                               v38 = icmp ugt v37, v36
;; @0031                               trapnz v38, user2
;;                                     v48 = iconst.i64 0
;; @0031                               v41 = icmp eq v14, v48  ; v48 = 0
;; @0031                               v26 = iconst.i64 8
;; @0031                               v39 = iadd v28, v53
;; @0031                               brif v41, block3, block2(v28)
;;
;;                                 block2(v42: i64):
;; @0031                               store.i64 user2 little region1 v4, v42
;;                                     v55 = iconst.i64 8
;;                                     v56 = iadd v42, v55  ; v55 = 8
;; @0031                               v45 = icmp eq v56, v39
;; @0031                               brif v45, block3, block2(v56)
;;
;;                                 block3:
;; @0034                               jump block1
;;
;;                                 block1:
;; @0034                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @003f                               trapz v2, user16
;; @003f                               v40 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003f                               v7 = load.i64 notrap aligned readonly can_move v40+32
;; @003f                               v6 = uextend.i64 v2
;; @003f                               v8 = iadd v7, v6
;; @003f                               v9 = iconst.i64 16
;; @003f                               v10 = iadd v8, v9  ; v9 = 16
;; @003f                               v11 = load.i32 user2 readonly region1 v10
;; @003f                               v13 = uextend.i64 v3
;; @003f                               v14 = uextend.i64 v4
;; @003f                               v17 = iadd v13, v14
;; @003f                               v12 = uextend.i64 v11
;; @003f                               v18 = icmp ugt v17, v12
;; @003f                               trapnz v18, user17
;; @003f                               v35 = load.i64 notrap aligned v40+40
;; @003f                               v23 = iconst.i64 24
;; @003f                               v24 = iadd v8, v23  ; v23 = 24
;;                                     v43 = iconst.i64 3
;;                                     v44 = ishl v13, v43  ; v43 = 3
;; @003f                               v28 = iadd v24, v44
;;                                     v46 = ishl v14, v43  ; v43 = 3
;; @003f                               v37 = uadd_overflow_trap v28, v46, user2
;; @003f                               v36 = iadd v7, v35
;; @003f                               v38 = icmp ugt v37, v36
;; @003f                               trapnz v38, user2
;; @003f                               v39 = iconst.i32 0
;; @003f                               call fn0(v0, v28, v39, v46)  ; v39 = 0
;; @0042                               jump block1
;;
;;                                 block1:
;; @0042                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @004d                               trapz v2, user16
;; @004d                               v40 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004d                               v7 = load.i64 notrap aligned readonly can_move v40+32
;; @004d                               v6 = uextend.i64 v2
;; @004d                               v8 = iadd v7, v6
;; @004d                               v9 = iconst.i64 16
;; @004d                               v10 = iadd v8, v9  ; v9 = 16
;; @004d                               v11 = load.i32 user2 readonly region1 v10
;; @004d                               v13 = uextend.i64 v3
;; @004d                               v14 = uextend.i64 v4
;; @004d                               v17 = iadd v13, v14
;; @004d                               v12 = uextend.i64 v11
;; @004d                               v18 = icmp ugt v17, v12
;; @004d                               trapnz v18, user17
;; @004d                               v35 = load.i64 notrap aligned v40+40
;; @004d                               v23 = iconst.i64 24
;; @004d                               v24 = iadd v8, v23  ; v23 = 24
;;                                     v44 = iconst.i64 3
;;                                     v45 = ishl v13, v44  ; v44 = 3
;; @004d                               v28 = iadd v24, v45
;;                                     v47 = ishl v14, v44  ; v44 = 3
;; @004d                               v37 = uadd_overflow_trap v28, v47, user2
;; @004d                               v36 = iadd v7, v35
;; @004d                               v38 = icmp ugt v37, v36
;; @004d                               trapnz v38, user2
;; @004d                               v39 = iconst.i32 255
;; @004d                               call fn0(v0, v28, v39, v47)  ; v39 = 255
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
;;
;; function u0:3(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @005b                               trapz v2, user16
;; @005b                               v46 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @005b                               v7 = load.i64 notrap aligned readonly can_move v46+32
;; @005b                               v6 = uextend.i64 v2
;; @005b                               v8 = iadd v7, v6
;; @005b                               v9 = iconst.i64 16
;; @005b                               v10 = iadd v8, v9  ; v9 = 16
;; @005b                               v11 = load.i32 user2 readonly region1 v10
;; @005b                               v13 = uextend.i64 v3
;; @005b                               v14 = uextend.i64 v4
;; @005b                               v17 = iadd v13, v14
;; @005b                               v12 = uextend.i64 v11
;; @005b                               v18 = icmp ugt v17, v12
;; @005b                               trapnz v18, user17
;; @005b                               v35 = load.i64 notrap aligned v46+40
;; @005b                               v23 = iconst.i64 24
;; @005b                               v24 = iadd v8, v23  ; v23 = 24
;;                                     v50 = iconst.i64 3
;;                                     v51 = ishl v13, v50  ; v50 = 3
;; @005b                               v28 = iadd v24, v51
;;                                     v53 = ishl v14, v50  ; v50 = 3
;; @005b                               v37 = uadd_overflow_trap v28, v53, user2
;; @005b                               v36 = iadd v7, v35
;; @005b                               v38 = icmp ugt v37, v36
;; @005b                               trapnz v38, user2
;;                                     v48 = iconst.i64 0
;; @005b                               v41 = icmp eq v14, v48  ; v48 = 0
;; @0057                               v5 = iconst.i64 1
;; @005b                               v26 = iconst.i64 8
;; @005b                               v39 = iadd v28, v53
;; @005b                               brif v41, block3, block2(v28)
;;
;;                                 block2(v42: i64):
;;                                     v55 = iconst.i64 1
;; @005b                               store user2 little region1 v55, v42  ; v55 = 1
;;                                     v56 = iconst.i64 8
;;                                     v57 = iadd v42, v56  ; v56 = 8
;; @005b                               v45 = icmp eq v57, v39
;; @005b                               brif v45, block3, block2(v57)
;;
;;                                 block3:
;; @005e                               jump block1
;;
;;                                 block1:
;; @005e                               return
;; }
