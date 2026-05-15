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
;; @0030                               v41 = load.i64 notrap aligned readonly can_move v0+8
;; @0030                               v7 = load.i64 notrap aligned readonly can_move v41+32
;; @0030                               v6 = uextend.i64 v2
;; @0030                               v8 = iadd v7, v6
;; @0030                               v9 = iconst.i64 24
;; @0030                               v10 = iadd v8, v9  ; v9 = 24
;; @0030                               v11 = load.i32 user2 readonly v10
;; @0030                               v12 = uadd_overflow_trap v3, v5, user17
;; @0030                               v13 = icmp ugt v12, v11
;; @0030                               trapnz v13, user17
;; @0030                               v15 = uextend.i64 v11
;;                                     v43 = iconst.i64 3
;;                                     v44 = ishl v15, v43  ; v43 = 3
;;                                     v40 = iconst.i64 32
;; @0030                               v17 = ushr v44, v40  ; v40 = 32
;; @0030                               trapnz v17, user2
;;                                     v53 = iconst.i32 3
;;                                     v54 = ishl v11, v53  ; v53 = 3
;; @0030                               v19 = iconst.i32 32
;; @0030                               v20 = uadd_overflow_trap v54, v19, user2  ; v19 = 32
;; @0030                               v24 = uadd_overflow_trap v2, v20, user2
;; @0030                               v25 = uextend.i64 v24
;; @0030                               v27 = iadd v7, v25
;;                                     v60 = ishl v3, v53  ; v53 = 3
;;                                     v62 = iadd v60, v19  ; v19 = 32
;; @0030                               v28 = isub v20, v62
;; @0030                               v29 = uextend.i64 v28
;; @0030                               v30 = isub v27, v29
;;                                     v64 = ishl v5, v53  ; v53 = 3
;; @0030                               v32 = uextend.i64 v64
;; @0030                               v33 = iadd v30, v32
;; @0030                               v14 = iconst.i64 8
;; @0030                               jump block2(v30)
;;
;;                                 block2(v35: i64):
;; @0030                               v36 = icmp eq v35, v33
;; @0030                               brif v36, block4, block3
;;
;;                                 block3:
;; @0030                               store.f64 user2 little v4, v35
;;                                     v66 = iconst.i64 8
;;                                     v67 = iadd.i64 v35, v66  ; v66 = 8
;; @0030                               jump block2(v67)
;;
;;                                 block4:
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
;;     fn0 = colocated u805306368:5 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0045                               trapz v2, user16
;; @0045                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @0045                               v7 = load.i64 notrap aligned readonly can_move v49+32
;; @0045                               v6 = uextend.i64 v2
;; @0045                               v8 = iadd v7, v6
;; @0045                               v9 = iconst.i64 24
;; @0045                               v10 = iadd v8, v9  ; v9 = 24
;; @0045                               v11 = load.i32 user2 readonly v10
;; @0045                               v12 = uadd_overflow_trap v3, v4, user17
;; @0045                               v13 = icmp ugt v12, v11
;; @0045                               trapnz v13, user17
;; @0045                               v15 = uextend.i64 v11
;;                                     v51 = iconst.i64 3
;;                                     v52 = ishl v15, v51  ; v51 = 3
;;                                     v48 = iconst.i64 32
;; @0045                               v17 = ushr v52, v48  ; v48 = 32
;; @0045                               trapnz v17, user2
;;                                     v61 = iconst.i32 3
;;                                     v62 = ishl v11, v61  ; v61 = 3
;; @0045                               v19 = iconst.i32 32
;; @0045                               v20 = uadd_overflow_trap v62, v19, user2  ; v19 = 32
;; @0045                               v24 = uadd_overflow_trap v2, v20, user2
;; @0045                               v37 = load.i64 notrap aligned v49+40
;; @0045                               v25 = uextend.i64 v24
;; @0045                               v27 = iadd v7, v25
;;                                     v68 = ishl v3, v61  ; v61 = 3
;;                                     v70 = iadd v68, v19  ; v19 = 32
;; @0045                               v28 = isub v20, v70
;; @0045                               v29 = uextend.i64 v28
;; @0045                               v30 = isub v27, v29
;;                                     v72 = ishl v4, v61  ; v61 = 3
;; @0045                               v32 = uextend.i64 v72
;; @0045                               v33 = iadd v30, v32
;; @0045                               v38 = iadd v7, v37
;; @0045                               v39 = icmp ugt v33, v38
;; @0045                               trapnz v39, user2
;; @0045                               v35 = iconst.i32 0
;; @0045                               call fn0(v0, v30, v35, v32)  ; v35 = 0
;; @0048                               jump block1
;;
;;                                 block1:
;; @0048                               return
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
;; @005a                               trapz v2, user16
;; @005a                               v41 = load.i64 notrap aligned readonly can_move v0+8
;; @005a                               v7 = load.i64 notrap aligned readonly can_move v41+32
;; @005a                               v6 = uextend.i64 v2
;; @005a                               v8 = iadd v7, v6
;; @005a                               v9 = iconst.i64 24
;; @005a                               v10 = iadd v8, v9  ; v9 = 24
;; @005a                               v11 = load.i32 user2 readonly v10
;; @005a                               v12 = uadd_overflow_trap v3, v4, user17
;; @005a                               v13 = icmp ugt v12, v11
;; @005a                               trapnz v13, user17
;; @005a                               v15 = uextend.i64 v11
;;                                     v43 = iconst.i64 3
;;                                     v44 = ishl v15, v43  ; v43 = 3
;;                                     v40 = iconst.i64 32
;; @005a                               v17 = ushr v44, v40  ; v40 = 32
;; @005a                               trapnz v17, user2
;;                                     v53 = iconst.i32 3
;;                                     v54 = ishl v11, v53  ; v53 = 3
;; @005a                               v19 = iconst.i32 32
;; @005a                               v20 = uadd_overflow_trap v54, v19, user2  ; v19 = 32
;; @005a                               v24 = uadd_overflow_trap v2, v20, user2
;; @005a                               v25 = uextend.i64 v24
;; @005a                               v27 = iadd v7, v25
;;                                     v60 = ishl v3, v53  ; v53 = 3
;;                                     v62 = iadd v60, v19  ; v19 = 32
;; @005a                               v28 = isub v20, v62
;; @005a                               v29 = uextend.i64 v28
;; @005a                               v30 = isub v27, v29
;;                                     v64 = ishl v4, v53  ; v53 = 3
;; @005a                               v32 = uextend.i64 v64
;; @005a                               v33 = iadd v30, v32
;; @004f                               v5 = f64const 0x1.0000000000000p0
;; @005a                               v14 = iconst.i64 8
;; @005a                               jump block2(v30)
;;
;;                                 block2(v35: i64):
;; @005a                               v36 = icmp eq v35, v33
;; @005a                               brif v36, block4, block3
;;
;;                                 block3:
;;                                     v66 = f64const 0x1.0000000000000p0
;; @005a                               store user2 little v66, v35  ; v66 = 0x1.0000000000000p0
;;                                     v67 = iconst.i64 8
;;                                     v68 = iadd.i64 v35, v67  ; v67 = 8
;; @005a                               jump block2(v68)
;;
;;                                 block4:
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return
;; }
