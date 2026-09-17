;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-W exceptions,function-references,gc -C collector=null"

;; Each exception payload field gets its own region, and the throwing and
;; catching functions must agree on them: `$throw`'s initializing stores and
;; `$catch`'s unboxing loads are the two halves of one region pair.

(module
  (tag $e (param i32 i64))

  (func $throw (param i32 i64)
    (throw $e (local.get 0) (local.get 1))
  )

  (func $catch (export "catch") (param i32 i64) (result i32 i64)
    (block $b (result i32 i64)
      (try_table (result i32 i64) (catch $e $b)
        (call $throw (local.get 0) (local.get 1))
        (i32.const 42)
        (i64.const 100)
      )
    )
  )
)

;; function u0:0(i64 vmctx, i64, i32, i64) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 65 ""
;;     region3 = 237 ""
;;     region4 = 206 ""
;;     region5 = 196 ""
;;     region6 = 130 ""
;;     region7 = 26 ""
;;     region8 = 108 ""
;;     region9 = 110 ""
;;     region10 = 142 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx) -> i32 tail
;;     sig1 = (i64 vmctx, i64) -> i8 tail
;;     sig2 = (i64 vmctx, i32) -> i8 tail
;;     fn0 = colocated u805306368:43 sig0
;;     fn1 = colocated u805306368:23 sig1
;;     fn2 = colocated u805306368:44 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64):
;; @003a                               v4 = call fn0(v0)
;; @003a                               v10 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @003a                               v11 = load.i32 notrap aligned region3 v10
;;                                     v45 = iconst.i32 7
;; @003a                               v14 = uadd_overflow_trap v11, v45, user18  ; v45 = 7
;;                                     v51 = iconst.i32 -8
;; @003a                               v16 = band v14, v51  ; v51 = -8
;; @003a                               v6 = iconst.i32 32
;; @003a                               v17 = uadd_overflow_trap v16, v6, user18  ; v6 = 32
;; @003a                               v19 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003a                               v20 = load.i64 notrap aligned region4 v19+40
;; @003a                               v18 = uextend.i64 v17
;; @003a                               v21 = icmp ule v18, v20
;; @003a                               brif v21, block2, block3
;;
;;                                 block2:
;;                                     v52 = iconst.i32 0x0400_0020
;; @003a                               v25 = load.i64 notrap aligned readonly can_move region5 v19+32
;;                                     v58 = band.i32 v14, v51  ; v51 = -8
;;                                     v59 = uextend.i64 v58
;; @003a                               v27 = iadd v25, v59
;; @003a                               store user2 region8 v52, v27  ; v52 = 0x0400_0020
;; @003a                               v30 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @003a                               v31 = load.i32 notrap aligned readonly can_move region7 v30+12
;; @003a                               store user2 region8 v31, v27+4
;; @003a                               store.i32 notrap aligned region3 v17, v10
;; @003a                               v32 = iconst.i64 16
;; @003a                               v33 = iadd v27, v32  ; v32 = 16
;; @003a                               store.i32 user2 little region9 v2, v33
;; @003a                               v34 = iconst.i64 24
;; @003a                               v35 = iadd v27, v34  ; v34 = 24
;; @003a                               store.i64 user2 little region10 v3, v35
;; @003a                               v36 = iconst.i64 8
;; @003a                               v37 = iadd v27, v36  ; v36 = 8
;; @003a                               store.i32 user2 little region8 v4, v37
;; @003a                               v5 = iconst.i32 0
;; @003a                               v38 = iconst.i64 12
;; @003a                               v39 = iadd v27, v38  ; v38 = 12
;; @003a                               store user2 little region8 v5, v39  ; v5 = 0
;; @003a                               try_call fn2(v0, v58), sig2, block4, [ context v0 ]
;;
;;                                 block4:
;; @003a                               trap user12
;;
;;                                 block3 cold:
;; @003a                               v22 = isub.i64 v18, v20
;; @003a                               v23 = call fn1(v0, v22)
;; @003a                               jump block2
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i64) -> i32, i64 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 196 ""
;;     region3 = 206 ""
;;     region4 = 110 ""
;;     region5 = 142 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u0:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64):
;; @0041                               jump block3
;;
;;                                 block5(v4: i64):
;; @0041                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0041                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @0041                               v5 = ireduce.i32 v4
;; @0041                               v6 = uextend.i64 v5
;; @0041                               v9 = iadd v8, v6
;; @0041                               v10 = iconst.i64 16
;; @0041                               v11 = iadd v9, v10  ; v10 = 16
;; @0041                               v12 = load.i32 user2 little region4 v11
;; @0041                               v17 = iconst.i64 24
;; @0041                               v18 = iadd v9, v17  ; v17 = 24
;; @0041                               v19 = load.i64 user2 little region5 v18
;; @0041                               jump block2(v12, v19)
;;
;;                                 block3:
;; @004b                               try_call fn0(v0, v0, v2, v3), sig0, block6, [ context v0, tag0: block5(exn0) ]
;;
;;                                 block6:
;; @0052                               jump block4
;;
;;                                 block4:
;; @004d                               v20 = iconst.i32 42
;; @004f                               v21 = iconst.i64 100
;; @0053                               jump block2(v20, v21)  ; v20 = 42, v21 = 100
;;
;;                                 block2(v22: i32, v23: i64):
;; @0054                               jump block1
;;
;;                                 block1:
;; @0054                               return v22, v23
;; }
