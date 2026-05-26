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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: f32, v5: i32):
;; @0030                               trapz v2, user16
;; @0030                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @0030                               v7 = load.i64 notrap aligned readonly can_move v49+32
;; @0030                               v6 = uextend.i64 v2
;; @0030                               v8 = iadd v7, v6
;; @0030                               v9 = iconst.i64 16
;; @0030                               v10 = iadd v8, v9  ; v9 = 16
;; @0030                               v11 = load.i32 user2 readonly v10
;; @0030                               v13 = uextend.i64 v3
;; @0030                               v14 = uextend.i64 v5
;; @0030                               v16 = iadd v13, v14
;; @0030                               v12 = uextend.i64 v11
;; @0030                               v17 = icmp ugt v16, v12
;; @0030                               trapnz v17, user17
;; @0030                               v28 = load.i64 notrap aligned v49+40
;;                                     v45 = iconst.i64 20
;; @0030                               v21 = iadd v8, v45  ; v45 = 20
;;                                     v53 = iconst.i64 2
;;                                     v54 = ishl v13, v53  ; v53 = 2
;; @0030                               v24 = iadd v21, v54
;;                                     v56 = ishl v14, v53  ; v53 = 2
;; @0030                               v30 = uadd_overflow_trap v24, v56, user2
;; @0030                               v29 = iadd v7, v28
;; @0030                               v31 = icmp ugt v30, v29
;; @0030                               trapnz v31, user2
;;                                     v51 = iconst.i64 0
;; @0030                               v33 = icmp eq v14, v51  ; v51 = 0
;;                                     v44 = iconst.i64 4
;; @0030                               v32 = iadd v24, v56
;; @0030                               brif v33, block3, block2(v24)
;;
;;                                 block2(v34: i64):
;; @0030                               store.f32 user2 little v4, v34
;;                                     v58 = iconst.i64 4
;;                                     v59 = iadd v34, v58  ; v58 = 4
;; @0030                               v36 = icmp eq v59, v32
;; @0030                               brif v36, block3, block2(v59)
;;
;;                                 block3:
;; @0033                               jump block1
;;
;;                                 block1:
;; @0033                               return
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
;; @0041                               trapz v2, user16
;; @0041                               v44 = load.i64 notrap aligned readonly can_move v0+8
;; @0041                               v7 = load.i64 notrap aligned readonly can_move v44+32
;; @0041                               v6 = uextend.i64 v2
;; @0041                               v8 = iadd v7, v6
;; @0041                               v9 = iconst.i64 16
;; @0041                               v10 = iadd v8, v9  ; v9 = 16
;; @0041                               v11 = load.i32 user2 readonly v10
;; @0041                               v13 = uextend.i64 v3
;; @0041                               v14 = uextend.i64 v4
;; @0041                               v16 = iadd v13, v14
;; @0041                               v12 = uextend.i64 v11
;; @0041                               v17 = icmp ugt v16, v12
;; @0041                               trapnz v17, user17
;; @0041                               v28 = load.i64 notrap aligned v44+40
;;                                     v40 = iconst.i64 20
;; @0041                               v21 = iadd v8, v40  ; v40 = 20
;;                                     v48 = iconst.i64 2
;;                                     v49 = ishl v13, v48  ; v48 = 2
;; @0041                               v24 = iadd v21, v49
;;                                     v51 = ishl v14, v48  ; v48 = 2
;; @0041                               v30 = uadd_overflow_trap v24, v51, user2
;; @0041                               v29 = iadd v7, v28
;; @0041                               v31 = icmp ugt v30, v29
;; @0041                               trapnz v31, user2
;; @0041                               v32 = iconst.i32 0
;; @0041                               call fn0(v0, v24, v32, v51)  ; v32 = 0
;; @0044                               jump block1
;;
;;                                 block1:
;; @0044                               return
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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0052                               trapz v2, user16
;; @0052                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @0052                               v7 = load.i64 notrap aligned readonly can_move v49+32
;; @0052                               v6 = uextend.i64 v2
;; @0052                               v8 = iadd v7, v6
;; @0052                               v9 = iconst.i64 16
;; @0052                               v10 = iadd v8, v9  ; v9 = 16
;; @0052                               v11 = load.i32 user2 readonly v10
;; @0052                               v13 = uextend.i64 v3
;; @0052                               v14 = uextend.i64 v4
;; @0052                               v16 = iadd v13, v14
;; @0052                               v12 = uextend.i64 v11
;; @0052                               v17 = icmp ugt v16, v12
;; @0052                               trapnz v17, user17
;; @0052                               v28 = load.i64 notrap aligned v49+40
;;                                     v45 = iconst.i64 20
;; @0052                               v21 = iadd v8, v45  ; v45 = 20
;;                                     v53 = iconst.i64 2
;;                                     v54 = ishl v13, v53  ; v53 = 2
;; @0052                               v24 = iadd v21, v54
;;                                     v56 = ishl v14, v53  ; v53 = 2
;; @0052                               v30 = uadd_overflow_trap v24, v56, user2
;; @0052                               v29 = iadd v7, v28
;; @0052                               v31 = icmp ugt v30, v29
;; @0052                               trapnz v31, user2
;;                                     v51 = iconst.i64 0
;; @0052                               v33 = icmp eq v14, v51  ; v51 = 0
;; @004b                               v5 = f32const 0x1.000000p0
;;                                     v44 = iconst.i64 4
;; @0052                               v32 = iadd v24, v56
;; @0052                               brif v33, block3, block2(v24)
;;
;;                                 block2(v34: i64):
;;                                     v58 = f32const 0x1.000000p0
;; @0052                               store user2 little v58, v34  ; v58 = 0x1.000000p0
;;                                     v59 = iconst.i64 4
;;                                     v60 = iadd v34, v59  ; v59 = 4
;; @0052                               v36 = icmp eq v60, v32
;; @0052                               brif v36, block3, block2(v60)
;;
;;                                 block3:
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return
;; }
