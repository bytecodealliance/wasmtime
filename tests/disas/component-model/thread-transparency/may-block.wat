;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[2]--function"
;;! flags = "-C inlining=y -Wconcurrency-support=y -Wcomponent-model-async=y"

;; $Mid lowers an `async`-typed function, which requires a `task_may_block`
;; check, so the $Outer -> $Mid adapter keeps its `{enter,exit}-sync-call`
;; calls.

(component
  (component $Inner
    (core module $M
      (func (export "g'") (result i32) unreachable)
    )
    (core instance $m (instantiate $M))
    (func (export "g") async (result u32)
      (canon lift (core func $m "g'"))
    )
  )

  (component $Mid
    (import "g" (func $g async (result u32)))
    (core func $g' (canon lower (func $g)))
    (core module $M
      (import "" "g'" (func $g' (result i32)))
      ;; What $Outer calls; it never reaches the `async`-typed lowering.
      (func (export "f'") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 34))
      )
      ;; The lowering is only reachable from this other export.
      (func (export "blocking") (result i32)
        (call $g')
      )
    )
    (core instance $m
      (instantiate $M
        (with "" (instance (export "g'" (func $g'))))
      )
    )
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'"))
    )
  )

  (component $Outer
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core module $M
      (import "" "f'" (func $f' (param i32) (result i32)))
      (func (export "g'") (result i32)
        (call $f' (i32.const 1200))
      )
    )
    (core instance $m
      (instantiate $M
        (with "" (instance (export "f'" (func $f'))))
      )
    )
    (func (export "g") (result u32)
      (canon lift (core func $m "g'"))
    )
  )

  (instance $inner (instantiate $Inner))
  (instance $mid (instantiate $Mid (with "g" (func $inner "g"))))
  (instance $outer (instantiate $Outer (with "f" (func $mid "f"))))

  (export "g" (func $outer "g"))
)
;; function u2:0(i64 vmctx, i64) -> i32 tail {
;;     ss0 = explicit_slot 24, align = 8
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 25 ""
;;     region3 = 44 ""
;;     region4 = 29 ""
;;     region5 = 68 ""
;;     region6 = 198 ""
;;     region7 = 100 ""
;;     region8 = 254 ""
;;     region9 = 112 ""
;;     region10 = 176 ""
;;     region11 = 149 ""
;;     region12 = 224 ""
;;     region13 = 78 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned region1 gv4+24
;;     gv6 = vmctx
;;     gv7 = load.i64 notrap aligned readonly can_move region0 gv6+8
;;     gv8 = load.i64 notrap aligned region1 gv7+24
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) tail
;;     sig2 = (i64 vmctx, i64, i32, i32) tail
;;     sig3 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u4:0 sig0
;;     fn1 = colocated u1:0 sig3
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @01d0                               jump block2
;;
;;                                 block2:
;;                                     jump block6
;;
;;                                 block6:
;; @01d0                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v9 = load.i64 notrap aligned readonly can_move region3 v3+232
;;                                     v10 = load.i32 notrap aligned region4 v9
;;                                     trapz v10, user26
;;                                     jump block9
;;
;;                                 block9:
;;                                     v15 = load.i64 notrap aligned readonly can_move region0 v3+8
;;                                     v16 = load.i64 notrap aligned region5 v15+136
;;                                     v14 = stack_addr.i64 ss0
;;                                     store notrap aligned region6 v16, v14
;;                                     v8 = iconst.i32 0
;;                                     store notrap aligned region7 v8, v14+8  ; v8 = 0
;;                                     v12 = iconst.i32 2
;;                                     store notrap aligned region8 v12, v14+12  ; v12 = 2
;;                                     v17 = load.i32 notrap aligned region9 v15+128
;;                                     store notrap aligned region10 v17, v14+16
;;                                     store notrap aligned region9 v8, v15+128  ; v8 = 0
;;                                     v19 = load.i32 notrap aligned region11 v15+132
;;                                     store notrap aligned region12 v19, v14+20
;;                                     store notrap aligned region11 v8, v15+132  ; v8 = 0
;;                                     store notrap aligned region5 v14, v15+136
;;                                     v21 = load.i64 notrap aligned readonly can_move region3 v3+208
;;                                     v22 = load.i32 notrap aligned region13 v21
;;                                     jump block16
;;
;;                                 block16:
;;                                     jump block17
;;
;;                                 block17:
;;                                     jump block11
;;
;;                                 block11:
;;                                     jump block12
;;
;;                                 block12:
;;                                     store.i64 notrap aligned region5 v16, v15+136
;;                                     store.i32 notrap aligned region9 v17, v15+128
;;                                     store.i32 notrap aligned region11 v19, v15+132
;;                                     jump block14
;;
;;                                 block14:
;;                                     store.i32 notrap aligned region4 v10, v9
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block4
;;
;;                                 block4:
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block18
;;
;;                                 block18:
;; @01d2                               jump block1
;;
;;                                 block1:
;;                                     v44 = iconst.i32 1234
;; @01d2                               return v44  ; v44 = 1234
;; }
