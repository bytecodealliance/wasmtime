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
;;     region1 = 160 ""
;;     region2 = 135 ""
;;     region3 = 215 ""
;;     region4 = 105 ""
;;     region5 = 133 ""
;;     region6 = 171 ""
;;     region7 = 153 ""
;;     sig0 = (i64 vmctx, i8) tail
;;     sig1 = (i64 vmctx) tail
;;     sig2 = (i64) preserve_all
;;     sig3 = (i64 vmctx, i32, i64, i32, i64, i64) -> i8 tail
;;     fn0 = colocated u805306368:40 sig0
;;     fn1 = colocated u805306368:41 sig1
;;     fn2 = colocated patchable u1073741824:46 sig2
;;     fn3 = colocated u805306368:51 sig3
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64):
;; @001d                               v4 = stack_addr.i64 ss0+8
;; @001d                               store notrap region2 v2, v4
;; @001d                               v5 = stack_addr.i64 ss0+12
;; @001d                               store notrap region2 v3, v5
;; @001d                               v6 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001d                               v7 = load.i64 notrap aligned region1 v6+24
;; @001d                               v8 = get_stack_pointer.i64 
;; @001d                               v9 = icmp ult v8, v7
;; @001d                               brif v9, block2, block3
;;
;;                                 block2 cold:
;; @0022                               v19 = iconst.i8 0
;; @001d                               call fn0(v0, v19)  ; v19 = 0
;; <ss0, 29, 4294967295> @001d         call fn1(v0)
;; @001d                               trap user1
;;
;;                                 block3:
;; @001d                               v11 = stack_addr.i64 ss0
;; @001d                               store.i64 notrap region2 v0, v11
;; <ss0, 30, 4294967295> @001e         call fn2(v0)
;; @001e                               v12 = stack_addr.i64 ss0+20
;; @001e                               store.i32 notrap region2 v2, v12
;; <ss0, 32, 0> @0020                  call fn2(v0)
;; @0020                               v13 = stack_addr.i64 ss0+24
;; @0020                               store.i64 notrap region2 v3, v13
;; <ss0, 34, 1> @0022                  call fn2(v0)
;; @0022                               v15 = load.i64 notrap aligned region4 v0+64
;; @0022                               v14 = uextend.i64 v2
;; @0022                               v16 = iconst.i64 16
;; @0022                               v17 = isub v15, v16  ; v16 = 16
;; @0022                               v18 = icmp ugt v14, v17
;; @0022                               brif v18, block4, block5
;;
;;                                 block4 cold:
;;                                     v54 = iconst.i8 1
;; @0022                               call fn0(v0, v54)  ; v54 = 1
;; <ss0, 34, 4294967295> @0022         call fn1(v0)
;; @0022                               trap user1
;;
;;                                 block5:
;; @0022                               v27 = load.i64 notrap aligned readonly can_move region5 v0+72
;; @0022                               v28 = load.i64 notrap aligned region4 v27+8
;;                                     v48 = iconst.i64 16
;;                                     v49 = isub v28, v48  ; v48 = 16
;; @0022                               v31 = icmp.i64 ugt v14, v49
;; @0022                               brif v31, block6, block7
;;
;;                                 block6 cold:
;; @0022                               v34 = iconst.i8 1
;; @0022                               call fn0(v0, v34)  ; v34 = 1
;; <ss0, 34, 4294967295> @0022         call fn1(v0)
;; @0022                               trap user1
;;
;;                                 block7:
;; @0022                               v36 = load.i64 notrap aligned readonly can_move region3 v27
;; @0022                               v37 = iadd v36, v14
;; @0022                               v24 = iconst.i64 8
;; @0022                               v39 = iadd v37, v24  ; v24 = 8
;; @0022                               v40 = load.i64 region7 v39
;; @0022                               brif v40, block8, block9
;;
;;                                 block8 cold:
;; @0022                               v42 = iconst.i32 0
;;                                     v50 = iconst.i64 8
;;                                     v51 = iadd.i64 v14, v50  ; v50 = 8
;; @0022                               v46 = iconst.i32 8
;; @0022                               v41 = iconst.i64 0
;; @0022                               v47 = call fn3(v0, v42, v51, v46, v3, v41)  ; v42 = 0, v46 = 8, v41 = 0
;; @0022                               jump block9
;;
;;                                 block9:
;; @0022                               v22 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @0022                               v23 = iadd v22, v14
;;                                     v52 = iconst.i64 8
;;                                     v53 = iadd v23, v52  ; v52 = 8
;; @0022                               store.i64 little region6 v3, v53
;; <ss0, 37, 4294967295> @0025         call fn2(v0)
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return
;; }
