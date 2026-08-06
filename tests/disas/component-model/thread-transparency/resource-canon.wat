;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[1]--function"
;;! flags = "-C inlining=y -Wconcurrency-support=y"

;; Any canonical built-in (here `resource.new`) makes the callee's instance
;; opaque.

(component
  (component $A
    (type $t (resource (rep i32)))
    (core func $new (canon resource.new $t))

    (core module $M
      (import "" "new" (func $new (param i32) (result i32)))
      (func (export "f'") (param i32) (result i32)
        (i32.add (local.get 0) (call $new (local.get 0)))
      )
    )

    (core instance $m
      (instantiate $M
        (with "" (instance (export "new" (func $new))))
      )
    )

    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'"))
    )
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))

    (core func $f' (canon lower (func $f)))

    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (func (export "g'") (result i32)
        (call $f' (i32.const 1234))
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
;;     ss0 = explicit_slot 24, align = 8
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1207959576 "VMFunctionImport+0x18"
;;     region3 = 1476395008 "VMGlobalImport+0x0"
;;     region4 = 738197568 "VMComponentContext+0x40"
;;     region5 = 67109000 "VMStoreContext+0x88"
;;     region6 = 1006632960 "VMDeferredThread+0x0"
;;     region7 = 1006632968 "VMDeferredThread+0x8"
;;     region8 = 1006632972 "VMDeferredThread+0xc"
;;     region9 = 67108992 "VMStoreContext+0x80"
;;     region10 = 1006632976 "VMDeferredThread+0x10"
;;     region11 = 67108996 "VMStoreContext+0x84"
;;     region12 = 1006632980 "VMDeferredThread+0x14"
;;     region13 = 738197552 "VMComponentContext+0x30"
;;     region14 = 1207959560 "VMFunctionImport+0x8"
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
;;     sig4 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u2:0 sig0
;;     fn1 = colocated u0:0 sig3
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0129                               jump block2
;;
;;                                 block2:
;;                                     jump block6
;;
;;                                 block8(v5: i64):
;;                                     jump block5
;;
;;                                 block6:
;; @0129                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v10 = load.i64 notrap aligned readonly can_move region3 v3+232
;;                                     v11 = load.i32 notrap aligned region4 v10
;;                                     trapz v11, user26
;;                                     jump block9
;;
;;                                 block9:
;;                                     v16 = load.i64 notrap aligned readonly can_move region0 v3+8
;;                                     v17 = load.i64 notrap aligned region5 v16+136
;;                                     v15 = stack_addr.i64 ss0
;;                                     store notrap aligned region6 v17, v15
;;                                     v9 = iconst.i32 0
;;                                     store notrap aligned region7 v9, v15+8  ; v9 = 0
;;                                     v13 = iconst.i32 1
;;                                     store notrap aligned region8 v13, v15+12  ; v13 = 1
;;                                     v18 = load.i32 notrap aligned region9 v16+128
;;                                     store notrap aligned region10 v18, v15+16
;;                                     store notrap aligned region9 v9, v16+128  ; v9 = 0
;;                                     v20 = load.i32 notrap aligned region11 v16+132
;;                                     store notrap aligned region12 v20, v15+20
;;                                     store notrap aligned region11 v9, v16+132  ; v9 = 0
;;                                     store notrap aligned region5 v15, v16+136
;;                                     v22 = load.i64 notrap aligned readonly can_move region3 v3+208
;;                                     v23 = load.i32 notrap aligned region13 v22
;;                                     jump block16
;;
;;                                 block16:
;;                                     v27 = load.i64 notrap aligned readonly can_move region2 v3+72
;;                                     v29 = load.i64 notrap aligned readonly can_move region14 v27+56
;;                                     v28 = load.i64 notrap aligned readonly can_move region2 v27+72
;; @0126                               v2 = iconst.i32 1234
;;                                     try_call_indirect v29(v28, v27, v2), sig4, block18(ret0), [ context v3, default: block8(exn0) ]  ; v2 = 1234
;;
;;                                 block18(v7: i32):
;;                                     jump block17
;;
;;                                 block17:
;;                                     jump block11
;;
;;                                 block11:
;;                                     v36 = load.i64 notrap aligned region5 v16+136
;;                                     v37 = icmp eq v36, v15
;;                                     brif v37, block12, block13
;;
;;                                 block12:
;;                                     v38 = load.i64 notrap aligned region6 v15
;;                                     store notrap aligned region5 v38, v16+136
;;                                     v39 = load.i32 notrap aligned region10 v15+16
;;                                     store notrap aligned region9 v39, v16+128
;;                                     v40 = load.i32 notrap aligned region12 v15+20
;;                                     store notrap aligned region11 v40, v16+132
;;                                     jump block14
;;
;;                                 block13:
;;                                     v44 = load.i64 notrap aligned readonly can_move region14 v3+152
;;                                     v33 = load.i64 notrap aligned readonly can_move region2 v3+168
;;                                     try_call_indirect v44(v33, v3), sig1, block15, [ context v3, default: block8(exn0) ]
;;
;;                                 block15:
;;                                     jump block14
;;
;;                                 block14:
;;                                     store.i32 notrap aligned region4 v11, v10
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block4
;;
;;                                 block5:
;;                                     trap user52
;;
;;                                 block4:
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block19
;;
;;                                 block19:
;; @012b                               jump block1
;;
;;                                 block1:
;;                                     v48 = iconst.i32 1234
;;                                     v49 = iadd.i32 v7, v48  ; v48 = 1234
;; @012b                               return v49
;; }
