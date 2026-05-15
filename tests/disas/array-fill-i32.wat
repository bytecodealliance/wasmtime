;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut i32)))

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
;; @0031                               v41 = load.i64 notrap aligned readonly can_move v0+8
;; @0031                               v7 = load.i64 notrap aligned readonly can_move v41+32
;; @0031                               v6 = uextend.i64 v2
;; @0031                               v8 = iadd v7, v6
;; @0031                               v9 = iconst.i64 24
;; @0031                               v10 = iadd v8, v9  ; v9 = 24
;; @0031                               v11 = load.i32 user2 readonly v10
;; @0031                               v12 = uadd_overflow_trap v3, v5, user17
;; @0031                               v13 = icmp ugt v12, v11
;; @0031                               trapnz v13, user17
;; @0031                               v15 = uextend.i64 v11
;;                                     v43 = iconst.i64 2
;;                                     v44 = ishl v15, v43  ; v43 = 2
;;                                     v40 = iconst.i64 32
;; @0031                               v17 = ushr v44, v40  ; v40 = 32
;; @0031                               trapnz v17, user2
;;                                     v53 = iconst.i32 2
;;                                     v54 = ishl v11, v53  ; v53 = 2
;; @0031                               v19 = iconst.i32 28
;; @0031                               v20 = uadd_overflow_trap v54, v19, user2  ; v19 = 28
;; @0031                               v24 = uadd_overflow_trap v2, v20, user2
;; @0031                               v25 = uextend.i64 v24
;; @0031                               v27 = iadd v7, v25
;;                                     v60 = ishl v3, v53  ; v53 = 2
;;                                     v62 = iadd v60, v19  ; v19 = 28
;; @0031                               v28 = isub v20, v62
;; @0031                               v29 = uextend.i64 v28
;; @0031                               v30 = isub v27, v29
;;                                     v64 = ishl v5, v53  ; v53 = 2
;; @0031                               v32 = uextend.i64 v64
;; @0031                               v33 = iadd v30, v32
;; @0031                               v14 = iconst.i64 4
;; @0031                               jump block2(v30)
;;
;;                                 block2(v35: i64):
;; @0031                               v36 = icmp eq v35, v33
;; @0031                               brif v36, block4, block3
;;
;;                                 block3:
;; @0031                               store.i32 user2 little v4, v35
;;                                     v66 = iconst.i64 4
;;                                     v67 = iadd.i64 v35, v66  ; v66 = 4
;; @0031                               jump block2(v67)
;;
;;                                 block4:
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
;;     fn0 = colocated u805306368:5 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @003f                               trapz v2, user16
;; @003f                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @003f                               v7 = load.i64 notrap aligned readonly can_move v49+32
;; @003f                               v6 = uextend.i64 v2
;; @003f                               v8 = iadd v7, v6
;; @003f                               v9 = iconst.i64 24
;; @003f                               v10 = iadd v8, v9  ; v9 = 24
;; @003f                               v11 = load.i32 user2 readonly v10
;; @003f                               v12 = uadd_overflow_trap v3, v4, user17
;; @003f                               v13 = icmp ugt v12, v11
;; @003f                               trapnz v13, user17
;; @003f                               v15 = uextend.i64 v11
;;                                     v51 = iconst.i64 2
;;                                     v52 = ishl v15, v51  ; v51 = 2
;;                                     v48 = iconst.i64 32
;; @003f                               v17 = ushr v52, v48  ; v48 = 32
;; @003f                               trapnz v17, user2
;;                                     v61 = iconst.i32 2
;;                                     v62 = ishl v11, v61  ; v61 = 2
;; @003f                               v19 = iconst.i32 28
;; @003f                               v20 = uadd_overflow_trap v62, v19, user2  ; v19 = 28
;; @003f                               v24 = uadd_overflow_trap v2, v20, user2
;; @003f                               v37 = load.i64 notrap aligned v49+40
;; @003f                               v25 = uextend.i64 v24
;; @003f                               v27 = iadd v7, v25
;;                                     v68 = ishl v3, v61  ; v61 = 2
;;                                     v70 = iadd v68, v19  ; v19 = 28
;; @003f                               v28 = isub v20, v70
;; @003f                               v29 = uextend.i64 v28
;; @003f                               v30 = isub v27, v29
;;                                     v72 = ishl v4, v61  ; v61 = 2
;; @003f                               v32 = uextend.i64 v72
;; @003f                               v33 = iadd v30, v32
;; @003f                               v38 = iadd v7, v37
;; @003f                               v39 = icmp ugt v33, v38
;; @003f                               trapnz v39, user2
;; @003b                               v5 = iconst.i32 0
;; @003f                               call fn0(v0, v30, v5, v32)  ; v5 = 0
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
;;     fn0 = colocated u805306368:5 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @004d                               trapz v2, user16
;; @004d                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @004d                               v7 = load.i64 notrap aligned readonly can_move v49+32
;; @004d                               v6 = uextend.i64 v2
;; @004d                               v8 = iadd v7, v6
;; @004d                               v9 = iconst.i64 24
;; @004d                               v10 = iadd v8, v9  ; v9 = 24
;; @004d                               v11 = load.i32 user2 readonly v10
;; @004d                               v12 = uadd_overflow_trap v3, v4, user17
;; @004d                               v13 = icmp ugt v12, v11
;; @004d                               trapnz v13, user17
;; @004d                               v15 = uextend.i64 v11
;;                                     v51 = iconst.i64 2
;;                                     v52 = ishl v15, v51  ; v51 = 2
;;                                     v48 = iconst.i64 32
;; @004d                               v17 = ushr v52, v48  ; v48 = 32
;; @004d                               trapnz v17, user2
;;                                     v61 = iconst.i32 2
;;                                     v62 = ishl v11, v61  ; v61 = 2
;; @004d                               v19 = iconst.i32 28
;; @004d                               v20 = uadd_overflow_trap v62, v19, user2  ; v19 = 28
;; @004d                               v24 = uadd_overflow_trap v2, v20, user2
;; @004d                               v37 = load.i64 notrap aligned v49+40
;; @004d                               v25 = uextend.i64 v24
;; @004d                               v27 = iadd v7, v25
;;                                     v68 = ishl v3, v61  ; v61 = 2
;;                                     v70 = iadd v68, v19  ; v19 = 28
;; @004d                               v28 = isub v20, v70
;; @004d                               v29 = uextend.i64 v28
;; @004d                               v30 = isub v27, v29
;;                                     v72 = ishl v4, v61  ; v61 = 2
;; @004d                               v32 = uextend.i64 v72
;; @004d                               v33 = iadd v30, v32
;; @004d                               v38 = iadd v7, v37
;; @004d                               v39 = icmp ugt v33, v38
;; @004d                               trapnz v39, user2
;; @004d                               v35 = iconst.i32 255
;; @004d                               call fn0(v0, v30, v35, v32)  ; v35 = 255
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
;; @005d                               v41 = load.i64 notrap aligned readonly can_move v0+8
;; @005d                               v7 = load.i64 notrap aligned readonly can_move v41+32
;; @005d                               v6 = uextend.i64 v2
;; @005d                               v8 = iadd v7, v6
;; @005d                               v9 = iconst.i64 24
;; @005d                               v10 = iadd v8, v9  ; v9 = 24
;; @005d                               v11 = load.i32 user2 readonly v10
;; @005d                               v12 = uadd_overflow_trap v3, v4, user17
;; @005d                               v13 = icmp ugt v12, v11
;; @005d                               trapnz v13, user17
;; @005d                               v15 = uextend.i64 v11
;;                                     v43 = iconst.i64 2
;;                                     v44 = ishl v15, v43  ; v43 = 2
;;                                     v40 = iconst.i64 32
;; @005d                               v17 = ushr v44, v40  ; v40 = 32
;; @005d                               trapnz v17, user2
;;                                     v53 = iconst.i32 2
;;                                     v54 = ishl v11, v53  ; v53 = 2
;; @005d                               v19 = iconst.i32 28
;; @005d                               v20 = uadd_overflow_trap v54, v19, user2  ; v19 = 28
;; @005d                               v24 = uadd_overflow_trap v2, v20, user2
;; @005d                               v25 = uextend.i64 v24
;; @005d                               v27 = iadd v7, v25
;;                                     v60 = ishl v3, v53  ; v53 = 2
;;                                     v62 = iadd v60, v19  ; v19 = 28
;; @005d                               v28 = isub v20, v62
;; @005d                               v29 = uextend.i64 v28
;; @005d                               v30 = isub v27, v29
;;                                     v64 = ishl v4, v53  ; v53 = 2
;; @005d                               v32 = uextend.i64 v64
;; @005d                               v33 = iadd v30, v32
;; @0057                               v5 = iconst.i32 0xdead
;; @005d                               v14 = iconst.i64 4
;; @005d                               jump block2(v30)
;;
;;                                 block2(v35: i64):
;; @005d                               v36 = icmp eq v35, v33
;; @005d                               brif v36, block4, block3
;;
;;                                 block3:
;;                                     v66 = iconst.i32 0xdead
;; @005d                               store user2 little v66, v35  ; v66 = 0xdead
;;                                     v67 = iconst.i64 4
;;                                     v68 = iadd.i64 v35, v67  ; v67 = 4
;; @005d                               jump block2(v68)
;;
;;                                 block4:
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
