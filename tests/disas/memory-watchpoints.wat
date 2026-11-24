;;! target = "x86_64"
;;! test = "optimize"
;;! flags = ["-Dguest-debug=yes"]

;; With guest debugging, each store first checks the memory's shadow bytes, a
;; bounds-checked access of the shadow memory at the same Wasm address, and
;; calls the watchpoint builtin if any are set.
(module
  (memory 1)
  (func (param i32 i64)
    (i64.store offset=8 (local.get 0) (local.get 1))))
;; function u0:0(i64 vmctx, i64, i32, i64) tail {
;;     ss0 = explicit_slot 32, key = 0
;;     region0 = 123 ""
;;     region1 = 25 ""
;;     region2 = 135 ""
;;     region3 = 215 ""
;;     region4 = 105 ""
;;     region5 = 133 ""
;;     region6 = 153 ""
;;     region7 = 171 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+16
;;     sig0 = (i64 vmctx) -> i8 tail
;;     sig1 = (i64) preserve_all
;;     sig2 = (i64 vmctx, i8) tail
;;     sig3 = (i64 vmctx) tail
;;     sig4 = (i64 vmctx, i32, i64, i32, i64, i64) -> i8 tail
;;     fn0 = colocated u805306368:41 sig0
;;     fn1 = colocated patchable u1073741824:46 sig1
;;     fn2 = colocated u805306368:39 sig2
;;     fn3 = colocated u805306368:40 sig3
;;     fn4 = colocated u805306368:51 sig4
;;     stack_limit = gv2, handler = fn0
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64):
;; @001d                               v4 = stack_addr.i64 ss0+8
;; @001d                               store notrap region2 v2, v4
;; @001d                               v5 = stack_addr.i64 ss0+12
;; @001d                               store notrap region2 v3, v5
;; @001d                               v6 = stack_addr.i64 ss0
;; @001d                               store notrap region2 v0, v6
;; <ss0, 30, 4294967295> @001e         call fn1(v0)
;; @001e                               v7 = stack_addr.i64 ss0+20
;; @001e                               store notrap region2 v2, v7
;; <ss0, 32, 0> @0020                  call fn1(v0)
;; @0020                               v8 = stack_addr.i64 ss0+24
;; @0020                               store notrap region2 v3, v8
;; <ss0, 34, 1> @0022                  call fn1(v0)
;; @0022                               v10 = load.i64 notrap aligned readonly can_move region5 v0+72
;; @0022                               v11 = load.i64 notrap aligned region4 v10+8
;; @0022                               v9 = uextend.i64 v2
;; @0022                               v12 = iconst.i64 16
;; @0022                               v13 = isub v11, v12  ; v12 = 16
;; @0022                               v14 = icmp ugt v9, v13
;; @0022                               brif v14, block2, block3
;;
;;                                 block2 cold:
;; @0022                               v49 = iconst.i8 1
;; <ss0, 34, 4294967295> @0022         call fn2(v0, v49)  ; v49 = 1
;; <ss0, 34, 4294967295> @0022         call fn3(v0)
;; @0022                               trap user1
;;
;;                                 block3:
;; @0022                               v19 = load.i64 notrap aligned readonly can_move region3 v10
;; @0022                               v20 = iadd v19, v9
;; @0022                               v21 = iconst.i64 8
;; @0022                               v22 = iadd v20, v21  ; v21 = 8
;; @0022                               v23 = load.i64 region6 v22
;; @0022                               brif v23, block4, block5
;;
;;                                 block4 cold:
;; @0022                               v25 = iconst.i32 0
;; @0022                               v43 = iconst.i64 8
;; @0022                               v44 = iadd.i64 v9, v43  ; v43 = 8
;; @0022                               v29 = iconst.i32 8
;; @0022                               v24 = iconst.i64 0
;; <ss0, 34, 4294967295> @0022         v30 = call fn4(v0, v25, v44, v29, v3, v24)  ; v25 = 0, v29 = 8, v24 = 0
;; @0022                               jump block5
;;
;;                                 block5:
;; @0022                               v32 = load.i64 notrap aligned region4 v0+64
;; @0022                               v45 = iconst.i64 16
;; @0022                               v46 = isub v32, v45  ; v45 = 16
;; @0022                               v35 = icmp.i64 ugt v9, v46
;; @0022                               brif v35, block6, block7
;;
;;                                 block6 cold:
;; @0022                               v38 = iconst.i8 1
;; <ss0, 34, 4294967295> @0022         call fn2(v0, v38)  ; v38 = 1
;; <ss0, 34, 4294967295> @0022         call fn3(v0)
;; @0022                               trap user1
;;
;;                                 block7:
;; @0022                               v39 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @0022                               v40 = iadd v39, v9
;; @0022                               v47 = iconst.i64 8
;; @0022                               v48 = iadd v40, v47  ; v47 = 8
;; @0022                               store.i64 little region7 v3, v48
;; <ss0, 37, 4294967295> @0025         call fn1(v0)
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return
;; }
