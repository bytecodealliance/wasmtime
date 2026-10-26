;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut funcref)))

  (func $fill-anything (param $a (ref $a)) (param $i i32) (param $v funcref) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )

  (func $fill-zero (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (ref.null func) (local.get $len))
  )

  (func $fill-bit-pattern (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (ref.func $hi) (local.get $len))
  )

  (func $hi)
  (elem declare func $hi)
)
;; function u0:0(i64 vmctx, i64, i32, i32, i64, i32) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 196 ""
;;     region3 = 206 ""
;;     region4 = 108 ""
;;     region5 = 5 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64, v5: i32):
;; @003b                               trapz v2, user16
;; @003b                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003b                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @003b                               v6 = uextend.i64 v2
;; @003b                               v9 = iadd v8, v6
;; @003b                               v10 = iconst.i64 16
;; @003b                               v11 = iadd v9, v10  ; v10 = 16
;; @003b                               v12 = load.i32 user2 readonly region4 v11
;; @003b                               v14 = uextend.i64 v3
;; @003b                               v15 = uextend.i64 v5
;; @003b                               v18 = iadd v14, v15
;; @003b                               v13 = uextend.i64 v12
;; @003b                               v19 = icmp ugt v18, v13
;; @003b                               trapnz v19, user17
;; @003b                               v36 = load.i64 notrap aligned region3 v7+40
;; @003b                               v24 = iconst.i64 20
;; @003b                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v51 = iconst.i64 2
;;                                     v52 = ishl v14, v51  ; v51 = 2
;; @003b                               v29 = iadd v25, v52
;;                                     v54 = ishl v15, v51  ; v51 = 2
;; @003b                               v38 = uadd_overflow_trap v29, v54, user2
;; @003b                               v37 = iadd v8, v36
;; @003b                               v39 = icmp ugt v38, v37
;; @003b                               trapnz v39, user2
;; @003b                               v40 = call fn0(v0, v4)
;;                                     v56 = iconst.i32 0
;;                                     v57 = icmp eq v5, v56  ; v56 = 0
;; @003b                               v41 = ireduce.i32 v40
;; @003b                               v27 = iconst.i64 4
;; @003b                               v42 = iadd v29, v54
;; @003b                               brif v57, block3, block2(v29)
;;
;;                                 block2(v45: i64):
;; @003b                               store.i32 notrap aligned little region5 v41, v45
;;                                     v59 = iconst.i64 4
;;                                     v60 = iadd v45, v59  ; v59 = 4
;; @003b                               v48 = icmp eq v60, v42
;; @003b                               brif v48, block3, block2(v60)
;;
;;                                 block3:
;; @003e                               jump block1
;;
;;                                 block1:
;; @003e                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 196 ""
;;     region3 = 206 ""
;;     region4 = 108 ""
;;     region5 = 5 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0049                               trapz v2, user16
;; @0049                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0049                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @0049                               v6 = uextend.i64 v2
;; @0049                               v9 = iadd v8, v6
;; @0049                               v10 = iconst.i64 16
;; @0049                               v11 = iadd v9, v10  ; v10 = 16
;; @0049                               v12 = load.i32 user2 readonly region4 v11
;; @0049                               v14 = uextend.i64 v3
;; @0049                               v15 = uextend.i64 v4
;; @0049                               v18 = iadd v14, v15
;; @0049                               v13 = uextend.i64 v12
;; @0049                               v19 = icmp ugt v18, v13
;; @0049                               trapnz v19, user17
;; @0049                               v36 = load.i64 notrap aligned region3 v7+40
;; @0049                               v24 = iconst.i64 20
;; @0049                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v50 = iconst.i64 2
;;                                     v51 = ishl v14, v50  ; v50 = 2
;; @0049                               v29 = iadd v25, v51
;;                                     v53 = ishl v15, v50  ; v50 = 2
;; @0049                               v38 = uadd_overflow_trap v29, v53, user2
;; @0049                               v37 = iadd v8, v36
;; @0049                               v39 = icmp ugt v38, v37
;; @0049                               trapnz v39, user2
;; @0045                               v5 = iconst.i64 0
;; @0049                               v40 = call fn0(v0, v5)  ; v5 = 0
;;                                     v55 = iconst.i32 0
;;                                     v56 = icmp eq v4, v55  ; v55 = 0
;; @0049                               v41 = ireduce.i32 v40
;; @0049                               v27 = iconst.i64 4
;; @0049                               v42 = iadd v29, v53
;; @0049                               brif v56, block3, block2(v29)
;;
;;                                 block2(v45: i64):
;; @0049                               store.i32 notrap aligned little region5 v41, v45
;;                                     v58 = iconst.i64 4
;;                                     v59 = iadd v45, v58  ; v58 = 4
;; @0049                               v48 = icmp eq v59, v42
;; @0049                               brif v48, block3, block2(v59)
;;
;;                                 block3:
;; @004c                               jump block1
;;
;;                                 block1:
;; @004c                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 130 ""
;;     region3 = 197 ""
;;     region4 = 44 ""
;;     region5 = 120 ""
;;     region6 = 100 ""
;;     region7 = 196 ""
;;     region8 = 206 ""
;;     region9 = 108 ""
;;     region10 = 5 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     sig1 = (i64 vmctx, i64, i64, i64) -> i8 system_v
;;     sig2 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u0:3 sig0
;;     fn1 = colocated u268435456:3 sig1
;;     fn2 = colocated u805306368:24 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0053                               v6 = func_addr.i64 fn1
;; @0053                               store notrap aligned region4 v6, v0+48
;; @0053                               v5 = func_addr.i64 fn0
;; @0053                               store notrap aligned region5 v5, v0+56
;; @0053                               store notrap aligned region6 v0, v0+72
;; @0053                               v7 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0053                               v8 = load.i32 notrap aligned readonly can_move region3 v7+20
;; @0053                               store notrap aligned region1 v8, v0+64
;; @0057                               trapz v2, user16
;; @0057                               v12 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0057                               v13 = load.i64 notrap aligned readonly can_move region7 v12+32
;; @0057                               v11 = uextend.i64 v2
;; @0057                               v14 = iadd v13, v11
;; @0057                               v15 = iconst.i64 16
;; @0057                               v16 = iadd v14, v15  ; v15 = 16
;; @0057                               v17 = load.i32 user2 readonly region9 v16
;; @0057                               v19 = uextend.i64 v3
;; @0057                               v20 = uextend.i64 v4
;; @0057                               v23 = iadd v19, v20
;; @0057                               v18 = uextend.i64 v17
;; @0057                               v24 = icmp ugt v23, v18
;; @0057                               trapnz v24, user17
;; @0057                               v41 = load.i64 notrap aligned region8 v12+40
;; @0057                               v29 = iconst.i64 20
;; @0057                               v30 = iadd v14, v29  ; v29 = 20
;;                                     v56 = iconst.i64 2
;;                                     v57 = ishl v19, v56  ; v56 = 2
;; @0057                               v34 = iadd v30, v57
;;                                     v59 = ishl v20, v56  ; v56 = 2
;; @0057                               v43 = uadd_overflow_trap v34, v59, user2
;; @0057                               v42 = iadd v13, v41
;; @0057                               v44 = icmp ugt v43, v42
;; @0057                               trapnz v44, user2
;; @0053                               v9 = iconst.i64 48
;; @0053                               v10 = iadd v0, v9  ; v9 = 48
;; @0057                               v45 = call fn2(v0, v10)
;;                                     v61 = iconst.i32 0
;;                                     v62 = icmp eq v4, v61  ; v61 = 0
;; @0057                               v46 = ireduce.i32 v45
;; @0057                               v32 = iconst.i64 4
;; @0057                               v47 = iadd v34, v59
;; @0057                               brif v62, block3, block2(v34)
;;
;;                                 block2(v50: i64):
;; @0057                               store.i32 notrap aligned little region10 v46, v50
;;                                     v64 = iconst.i64 4
;;                                     v65 = iadd v50, v64  ; v64 = 4
;; @0057                               v53 = icmp eq v65, v47
;; @0057                               brif v53, block3, block2(v65)
;;
;;                                 block3:
;; @005a                               jump block1
;;
;;                                 block1:
;; @005a                               return
;; }
;;
;; function u0:3(i64 vmctx, i64) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return
;; }
