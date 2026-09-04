;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[1]--function"
;;! flags = "-C inlining=y -Wconcurrency-support=y"

;; Only handles disqualify an adapter's signature, so an aggregate parameter is
;; still thread-transparent and the `enter-sync-call`/`exit-sync-call` calls are
;; omitted below.

(component
  (component $A
    (core module $M
      ;; The lifted signature flattens to a discriminant plus the tuple's fields.
      (func (export "f'") (param i32 i32 i32) (result i32)
        (if (result i32) (local.get 0)
          (then (i32.add (local.get 1) (local.get 2)))
          (else (i32.const 0)))
      )
    )

    (core instance $m (instantiate $M))

    (func (export "f") (param "x" (option (tuple u32 u32))) (result u32)
      (canon lift (core func $m "f'"))
    )
  )

  (component $B
    (import "f" (func $f (param "x" (option (tuple u32 u32))) (result u32)))

    (core func $f' (canon lower (func $f)))

    (core module $N
      (import "" "f'" (func $f' (param i32 i32 i32) (result i32)))
      (func (export "g'") (result i32)
        (call $f' (i32.const 1) (i32.const 1200) (i32.const 34))
      )
    )

    (core instance $n
      (instantiate $N
        (with "" (instance (export "f'" (func $f'))))
      )
    )

    (func (export "g") (result u32)
      (canon lift (core func $n "g'"))
    )
  )

  (instance $a (instantiate $A))
  (instance $b
    (instantiate $B
      (with "f" (func $a "f"))
    )
  )

  (export "g" (func $b "g"))
)
;; function u1:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1207959576 "VMFunctionImport+0x18"
;;     region3 = 1476395008 "VMGlobalImport+0x0"
;;     region4 = 738197568 "VMComponentContext+0x40"
;;     region5 = 738197552 "VMComponentContext+0x30"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned region1 gv4+24
;;     gv6 = vmctx
;;     gv7 = load.i64 notrap aligned readonly can_move region0 gv6+8
;;     gv8 = load.i64 notrap aligned region1 gv7+24
;;     sig0 = (i64 vmctx, i64, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) tail
;;     sig2 = (i64 vmctx, i64, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u2:0 sig0
;;     fn1 = colocated u0:0 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @010a                               jump block2
;;
;;                                 block2:
;;                                     jump block6
;;
;;                                 block6:
;; @010a                               v5 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v15 = load.i64 notrap aligned readonly can_move region3 v5+200
;;                                     v16 = load.i32 notrap aligned region4 v15
;;                                     trapz v16, user26
;;                                     jump block9
;;
;;                                 block9:
;;                                     v17 = load.i64 notrap aligned readonly can_move region3 v5+176
;;                                     v18 = load.i32 notrap aligned region5 v17
;;                                     v14 = iconst.i32 0
;;                                     store notrap aligned region5 v14, v17  ; v14 = 0
;;                                     jump block12
;;
;;                                 block12:
;; @0103                               v2 = iconst.i32 1
;; @0105                               v3 = iconst.i32 1200
;; @0108                               v4 = iconst.i32 34
;;                                     jump block11(v2, v3, v4)  ; v2 = 1, v3 = 1200, v4 = 34
;;
;;                                 block11(v8: i32, v9: i32, v10: i32):
;;                                     store.i32 notrap aligned region5 v18, v17
;;                                     jump block16
;;
;;                                 block16:
;;                                     brif.i32 v8, block18, block20
;;
;;                                 block18:
;;                                     v27 = iadd.i32 v9, v10
;;                                     jump block19(v27)
;;
;;                                 block20:
;;                                     v34 = iconst.i32 0
;;                                     jump block19(v34)  ; v34 = 0
;;
;;                                 block19(v12: i32):
;;                                     jump block17
;;
;;                                 block17:
;;                                     jump block15(v12)
;;
;;                                 block15(v11: i32):
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block4
;;
;;                                 block4:
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block21(v11)
;;
;;                                 block21(v13: i32):
;; @010c                               jump block1
;;
;;                                 block1:
;; @010c                               return v13
;; }
