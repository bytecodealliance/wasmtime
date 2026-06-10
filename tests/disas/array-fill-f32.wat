;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut f32)))

  (func $fill-anything (param $a (ref $a)) (param $i i32) (param $v f32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )

  (func $fill-zero (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (f32.const 0) (local.get $len))
  )

  (func $fill-bit-pattern (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (f32.const 1) (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, f32, i32) tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: f32, v5: i32):
;; @0030                               trapz v2, user16
;; @0030                               v46 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0030                               v7 = load.i64 notrap aligned readonly can_move v46+32
;; @0030                               v6 = uextend.i64 v2
;; @0030                               v8 = iadd v7, v6
;; @0030                               v9 = iconst.i64 16
;; @0030                               v10 = iadd v8, v9  ; v9 = 16
;; @0030                               v11 = load.i32 user2 readonly region1 v10
;; @0030                               v13 = uextend.i64 v3
;; @0030                               v14 = uextend.i64 v5
;; @0030                               v17 = iadd v13, v14
;; @0030                               v12 = uextend.i64 v11
;; @0030                               v18 = icmp ugt v17, v12
;; @0030                               trapnz v18, user17
;; @0030                               v35 = load.i64 notrap aligned v46+40
;; @0030                               v23 = iconst.i64 20
;; @0030                               v24 = iadd v8, v23  ; v23 = 20
;;                                     v50 = iconst.i64 2
;;                                     v51 = ishl v13, v50  ; v50 = 2
;; @0030                               v28 = iadd v24, v51
;;                                     v53 = ishl v14, v50  ; v50 = 2
;; @0030                               v37 = uadd_overflow_trap v28, v53, user2
;; @0030                               v36 = iadd v7, v35
;; @0030                               v38 = icmp ugt v37, v36
;; @0030                               trapnz v38, user2
;;                                     v48 = iconst.i64 0
;; @0030                               v41 = icmp eq v14, v48  ; v48 = 0
;; @0030                               v26 = iconst.i64 4
;; @0030                               v39 = iadd v28, v53
;; @0030                               brif v41, block3, block2(v28)
;;
;;                                 block2(v42: i64):
;; @0030                               store.f32 user2 little region1 v4, v42
;;                                     v55 = iconst.i64 4
;;                                     v56 = iadd v42, v55  ; v55 = 4
;; @0030                               v45 = icmp eq v56, v39
;; @0030                               brif v45, block3, block2(v56)
;;
;;                                 block3:
;; @0033                               jump block1
;;
;;                                 block1:
;; @0033                               return
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
;; @0041                               trapz v2, user16
;; @0041                               v40 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0041                               v7 = load.i64 notrap aligned readonly can_move v40+32
;; @0041                               v6 = uextend.i64 v2
;; @0041                               v8 = iadd v7, v6
;; @0041                               v9 = iconst.i64 16
;; @0041                               v10 = iadd v8, v9  ; v9 = 16
;; @0041                               v11 = load.i32 user2 readonly region1 v10
;; @0041                               v13 = uextend.i64 v3
;; @0041                               v14 = uextend.i64 v4
;; @0041                               v17 = iadd v13, v14
;; @0041                               v12 = uextend.i64 v11
;; @0041                               v18 = icmp ugt v17, v12
;; @0041                               trapnz v18, user17
;; @0041                               v35 = load.i64 notrap aligned v40+40
;; @0041                               v23 = iconst.i64 20
;; @0041                               v24 = iadd v8, v23  ; v23 = 20
;;                                     v44 = iconst.i64 2
;;                                     v45 = ishl v13, v44  ; v44 = 2
;; @0041                               v28 = iadd v24, v45
;;                                     v47 = ishl v14, v44  ; v44 = 2
;; @0041                               v37 = uadd_overflow_trap v28, v47, user2
;; @0041                               v36 = iadd v7, v35
;; @0041                               v38 = icmp ugt v37, v36
;; @0041                               trapnz v38, user2
;; @0041                               v39 = iconst.i32 0
;; @0041                               call fn0(v0, v28, v39, v47)  ; v39 = 0
;; @0044                               jump block1
;;
;;                                 block1:
;; @0044                               return
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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0052                               trapz v2, user16
;; @0052                               v46 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0052                               v7 = load.i64 notrap aligned readonly can_move v46+32
;; @0052                               v6 = uextend.i64 v2
;; @0052                               v8 = iadd v7, v6
;; @0052                               v9 = iconst.i64 16
;; @0052                               v10 = iadd v8, v9  ; v9 = 16
;; @0052                               v11 = load.i32 user2 readonly region1 v10
;; @0052                               v13 = uextend.i64 v3
;; @0052                               v14 = uextend.i64 v4
;; @0052                               v17 = iadd v13, v14
;; @0052                               v12 = uextend.i64 v11
;; @0052                               v18 = icmp ugt v17, v12
;; @0052                               trapnz v18, user17
;; @0052                               v35 = load.i64 notrap aligned v46+40
;; @0052                               v23 = iconst.i64 20
;; @0052                               v24 = iadd v8, v23  ; v23 = 20
;;                                     v50 = iconst.i64 2
;;                                     v51 = ishl v13, v50  ; v50 = 2
;; @0052                               v28 = iadd v24, v51
;;                                     v53 = ishl v14, v50  ; v50 = 2
;; @0052                               v37 = uadd_overflow_trap v28, v53, user2
;; @0052                               v36 = iadd v7, v35
;; @0052                               v38 = icmp ugt v37, v36
;; @0052                               trapnz v38, user2
;;                                     v48 = iconst.i64 0
;; @0052                               v41 = icmp eq v14, v48  ; v48 = 0
;; @004b                               v5 = f32const 0x1.000000p0
;; @0052                               v26 = iconst.i64 4
;; @0052                               v39 = iadd v28, v53
;; @0052                               brif v41, block3, block2(v28)
;;
;;                                 block2(v42: i64):
;;                                     v55 = f32const 0x1.000000p0
;; @0052                               store user2 little region1 v55, v42  ; v55 = 0x1.000000p0
;;                                     v56 = iconst.i64 4
;;                                     v57 = iadd v42, v56  ; v56 = 4
;; @0052                               v45 = icmp eq v57, v39
;; @0052                               brif v45, block3, block2(v57)
;;
;;                                 block3:
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return
;; }
