;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut i16)))

  (func $fill-anything (param $a (ref $a)) (param $i i32) (param $v i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )

  (func $fill-zero (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (i32.const 0) (local.get $len))
  )

  (func $fill-minus-one (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (i32.const -1) (local.get $len))
  )

  (func $fill-bit-pattern (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (i32.const 0xdead) (local.get $len))
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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0031                               trapz v2, user16
;; @0031                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @0031                               v7 = load.i64 notrap aligned readonly can_move v49+32
;; @0031                               v6 = uextend.i64 v2
;; @0031                               v8 = iadd v7, v6
;; @0031                               v9 = iconst.i64 16
;; @0031                               v10 = iadd v8, v9  ; v9 = 16
;; @0031                               v11 = load.i32 user2 readonly v10
;; @0031                               v13 = uextend.i64 v3
;; @0031                               v14 = uextend.i64 v5
;; @0031                               v16 = iadd v13, v14
;; @0031                               v12 = uextend.i64 v11
;; @0031                               v17 = icmp ugt v16, v12
;; @0031                               trapnz v17, user17
;; @0031                               v28 = load.i64 notrap aligned v49+40
;;                                     v45 = iconst.i64 20
;; @0031                               v21 = iadd v8, v45  ; v45 = 20
;;                                     v48 = iconst.i64 1
;;                                     v54 = ishl v13, v48  ; v48 = 1
;; @0031                               v24 = iadd v21, v54
;;                                     v58 = ishl v14, v48  ; v48 = 1
;; @0031                               v30 = uadd_overflow_trap v24, v58, user2
;; @0031                               v29 = iadd v7, v28
;; @0031                               v31 = icmp ugt v30, v29
;; @0031                               trapnz v31, user2
;;                                     v51 = iconst.i64 0
;; @0031                               v33 = icmp eq v14, v51  ; v51 = 0
;;                                     v44 = iconst.i64 2
;; @0031                               v32 = iadd v24, v58
;; @0031                               brif v33, block3, block2(v24)
;;
;;                                 block2(v34: i64):
;; @0031                               istore16.i32 user2 little v4, v34
;;                                     v61 = iconst.i64 2
;;                                     v62 = iadd v34, v61  ; v61 = 2
;; @0031                               v36 = icmp eq v62, v32
;; @0031                               brif v36, block3, block2(v62)
;;
;;                                 block3:
;; @0034                               jump block1
;;
;;                                 block1:
;; @0034                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32) tail {
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
;; @003f                               v44 = load.i64 notrap aligned readonly can_move v0+8
;; @003f                               v7 = load.i64 notrap aligned readonly can_move v44+32
;; @003f                               v6 = uextend.i64 v2
;; @003f                               v8 = iadd v7, v6
;; @003f                               v9 = iconst.i64 16
;; @003f                               v10 = iadd v8, v9  ; v9 = 16
;; @003f                               v11 = load.i32 user2 readonly v10
;; @003f                               v13 = uextend.i64 v3
;; @003f                               v14 = uextend.i64 v4
;; @003f                               v16 = iadd v13, v14
;; @003f                               v12 = uextend.i64 v11
;; @003f                               v17 = icmp ugt v16, v12
;; @003f                               trapnz v17, user17
;; @003f                               v28 = load.i64 notrap aligned v44+40
;;                                     v40 = iconst.i64 20
;; @003f                               v21 = iadd v8, v40  ; v40 = 20
;;                                     v43 = iconst.i64 1
;;                                     v49 = ishl v13, v43  ; v43 = 1
;; @003f                               v24 = iadd v21, v49
;;                                     v53 = ishl v14, v43  ; v43 = 1
;; @003f                               v30 = uadd_overflow_trap v24, v53, user2
;; @003f                               v29 = iadd v7, v28
;; @003f                               v31 = icmp ugt v30, v29
;; @003f                               trapnz v31, user2
;; @003b                               v5 = iconst.i32 0
;; @003f                               call fn0(v0, v24, v5, v53)  ; v5 = 0
;; @0042                               jump block1
;;
;;                                 block1:
;; @0042                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32) tail {
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
;; @004d                               v44 = load.i64 notrap aligned readonly can_move v0+8
;; @004d                               v7 = load.i64 notrap aligned readonly can_move v44+32
;; @004d                               v6 = uextend.i64 v2
;; @004d                               v8 = iadd v7, v6
;; @004d                               v9 = iconst.i64 16
;; @004d                               v10 = iadd v8, v9  ; v9 = 16
;; @004d                               v11 = load.i32 user2 readonly v10
;; @004d                               v13 = uextend.i64 v3
;; @004d                               v14 = uextend.i64 v4
;; @004d                               v16 = iadd v13, v14
;; @004d                               v12 = uextend.i64 v11
;; @004d                               v17 = icmp ugt v16, v12
;; @004d                               trapnz v17, user17
;; @004d                               v28 = load.i64 notrap aligned v44+40
;;                                     v40 = iconst.i64 20
;; @004d                               v21 = iadd v8, v40  ; v40 = 20
;;                                     v43 = iconst.i64 1
;;                                     v49 = ishl v13, v43  ; v43 = 1
;; @004d                               v24 = iadd v21, v49
;;                                     v53 = ishl v14, v43  ; v43 = 1
;; @004d                               v30 = uadd_overflow_trap v24, v53, user2
;; @004d                               v29 = iadd v7, v28
;; @004d                               v31 = icmp ugt v30, v29
;; @004d                               trapnz v31, user2
;; @004d                               v32 = iconst.i32 255
;; @004d                               call fn0(v0, v24, v32, v53)  ; v32 = 255
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
;;
;; function u0:3(i64 vmctx, i64, i32, i32, i32) tail {
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
;; @005d                               trapz v2, user16
;; @005d                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @005d                               v7 = load.i64 notrap aligned readonly can_move v49+32
;; @005d                               v6 = uextend.i64 v2
;; @005d                               v8 = iadd v7, v6
;; @005d                               v9 = iconst.i64 16
;; @005d                               v10 = iadd v8, v9  ; v9 = 16
;; @005d                               v11 = load.i32 user2 readonly v10
;; @005d                               v13 = uextend.i64 v3
;; @005d                               v14 = uextend.i64 v4
;; @005d                               v16 = iadd v13, v14
;; @005d                               v12 = uextend.i64 v11
;; @005d                               v17 = icmp ugt v16, v12
;; @005d                               trapnz v17, user17
;; @005d                               v28 = load.i64 notrap aligned v49+40
;;                                     v45 = iconst.i64 20
;; @005d                               v21 = iadd v8, v45  ; v45 = 20
;;                                     v48 = iconst.i64 1
;;                                     v54 = ishl v13, v48  ; v48 = 1
;; @005d                               v24 = iadd v21, v54
;;                                     v58 = ishl v14, v48  ; v48 = 1
;; @005d                               v30 = uadd_overflow_trap v24, v58, user2
;; @005d                               v29 = iadd v7, v28
;; @005d                               v31 = icmp ugt v30, v29
;; @005d                               trapnz v31, user2
;;                                     v51 = iconst.i64 0
;; @005d                               v33 = icmp eq v14, v51  ; v51 = 0
;; @0057                               v5 = iconst.i32 0xdead
;;                                     v44 = iconst.i64 2
;; @005d                               v32 = iadd v24, v58
;; @005d                               brif v33, block3, block2(v24)
;;
;;                                 block2(v34: i64):
;;                                     v61 = iconst.i32 0xdead
;; @005d                               istore16 user2 little v61, v34  ; v61 = 0xdead
;;                                     v62 = iconst.i64 2
;;                                     v63 = iadd v34, v62  ; v62 = 2
;; @005d                               v36 = icmp eq v63, v32
;; @005d                               brif v36, block3, block2(v63)
;;
;;                                 block3:
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
