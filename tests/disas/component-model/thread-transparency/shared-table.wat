;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[3]--function"
;;! flags = "-C inlining=y -Wconcurrency-support=y"

;; The analysis works at component-instance rather than core-instance
;; granularity: $Inner's lifted callee imports nothing but a core table, but a
;; sibling core instance planted a `context`-poking function in it that
;; `call_indirect` reaches. $Inner declares `canon context.{get,set}`, so the
;; $Outer -> $Inner adapter is opaque.

(component
  (component $Inner
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))

    (core module $Shared (table (export "t") 1 funcref))
    (core instance $shared (instantiate $Shared))

    (core module $Evil
      (import "" "t" (table 1 funcref))
      (import "" "cget" (func $cget (result i32)))
      (import "" "cset" (func $cset (param i32)))
      (func $leak (result i32)
        (call $cset (i32.const 0x5555))
        (call $cget))
      (elem (table 0) (i32.const 0) func $leak)
    )
    (core instance $evil
      (instantiate $Evil
        (with "" (instance
          (export "t" (table $shared "t"))
          (export "cget" (func $cget))
          (export "cset" (func $cset))
        ))
      )
    )

    (core module $Victim
      (import "" "t" (table 1 funcref))
      (type $sig (func (result i32)))
      (func (export "f'") (result i32)
        (call_indirect (type $sig) (i32.const 0)))
    )
    (core instance $victim
      (instantiate $Victim
        (with "" (instance (export "t" (table $shared "t"))))
      )
    )

    (func (export "f") (result u32)
      (canon lift (core func $victim "f'"))
    )
  )

  (component $Outer
    (import "f" (func $f (result u32)))
    (core func $f' (canon lower (func $f)))
    (core module $M
      (import "" "f'" (func $f' (result i32)))
      (func (export "g'") (result i32)
        (call $f')
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
  (instance $outer (instantiate $Outer (with "f" (func $inner "f"))))

  (export "g" (func $outer "g"))
)
;; function u3:0(i64 vmctx, i64) -> i32 tail {
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
;;     region14 = 1342177280 "VMTableImport+0x0"
;;     region15 = 671088648 "VMTableDefinition+0x8"
;;     region16 = 671088640 "VMTableDefinition+0x0"
;;     region17 = 335544320 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     region18 = 40 "VMContext+0x28"
;;     region19 = 1677721600 "TypeIdsArray+0x0"
;;     region20 = 1610612752 "VMFuncRef+0x10"
;;     region21 = 1610612744 "VMFuncRef+0x8"
;;     region22 = 1610612760 "VMFuncRef+0x18"
;;     region23 = 1207959560 "VMFunctionImport+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned region1 gv4+24
;;     gv6 = vmctx
;;     gv7 = load.i64 notrap aligned readonly can_move region0 gv6+8
;;     gv8 = load.i64 notrap aligned region1 gv7+24
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i64) tail
;;     sig2 = (i64 vmctx, i64, i32, i32) tail
;;     sig3 = (i64 vmctx, i64) -> i32 tail
;;     sig4 = (i64 vmctx, i64) -> i32 tail
;;     sig5 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u4:0 sig0
;;     fn1 = colocated u2:0 sig3
;;     fn2 = colocated u805306368:7 sig5
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @020e                               jump block2
;;
;;                                 block2:
;;                                     jump block6
;;
;;                                 block8(v4: i64):
;;                                     jump block5
;;
;;                                 block6:
;; @020e                               v2 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v11 = load.i64 notrap aligned readonly can_move region3 v2+232
;;                                     v12 = load.i32 notrap aligned region4 v11
;;                                     trapz v12, user26
;;                                     jump block9
;;
;;                                 block9:
;;                                     v17 = load.i64 notrap aligned readonly can_move region0 v2+8
;;                                     v18 = load.i64 notrap aligned region5 v17+136
;;                                     v16 = stack_addr.i64 ss0
;;                                     store notrap aligned region6 v18, v16
;;                                     v10 = iconst.i32 0
;;                                     store notrap aligned region7 v10, v16+8  ; v10 = 0
;;                                     v14 = iconst.i32 1
;;                                     store notrap aligned region8 v14, v16+12  ; v14 = 1
;;                                     v19 = load.i32 notrap aligned region9 v17+128
;;                                     store notrap aligned region10 v19, v16+16
;;                                     store notrap aligned region9 v10, v17+128  ; v10 = 0
;;                                     v21 = load.i32 notrap aligned region11 v17+132
;;                                     store notrap aligned region12 v21, v16+20
;;                                     store notrap aligned region11 v10, v17+132  ; v10 = 0
;;                                     store notrap aligned region5 v16, v17+136
;;                                     v23 = load.i64 notrap aligned readonly can_move region3 v2+208
;;                                     v24 = load.i32 notrap aligned region13 v23
;;                                     jump block16
;;
;;                                 block16:
;;                                     v28 = load.i64 notrap aligned readonly can_move region2 v2+72
;;                                     v30 = load.i64 notrap aligned readonly can_move region14 v28+48
;;                                     v31 = load.i64 notrap aligned region15 v30+8
;;                                     v36 = load.i64 notrap aligned region16 v30
;;                                     v32 = ireduce.i32 v31
;;                                     v74 = iconst.i32 0
;;                                     v75 = icmp eq v32, v74  ; v74 = 0
;;                                     v73 = iconst.i64 0
;;                                     v41 = select_spectre_guard v75, v73, v36  ; v73 = 0
;;                                     v42 = load.i64 user6 aligned region17 v41
;;                                     v43 = iconst.i64 -2
;;                                     v44 = band v42, v43  ; v43 = -2
;;                                     brif v42, block19(v44), block18
;;
;;                                 block18 cold:
;;                                     v76 = iconst.i32 0
;;                                     v77 = iconst.i64 0
;;                                     try_call fn2(v28, v76, v77), sig5, block21(ret0), [ context v2, default: block8(exn0) ]  ; v76 = 0, v77 = 0
;;
;;                                 block21(v8: i64):
;;                                     jump block19(v8)
;;
;;                                 block19(v6: i64):
;;                                     v47 = load.i32 user7 aligned readonly region20 v6+16
;;                                     v45 = load.i64 notrap aligned readonly can_move region18 v28+40
;;                                     v46 = load.i32 notrap aligned readonly can_move region19 v45
;;                                     v48 = icmp eq v47, v46
;;                                     trapz v48, user8
;;                                     v50 = load.i64 notrap aligned readonly region21 v6+8
;;                                     v51 = load.i64 notrap aligned readonly region22 v6+24
;;                                     try_call_indirect v50(v51, v28), sig4, block20(ret0), [ context v2, default: block8(exn0) ]
;;
;;                                 block20(v7: i32):
;;                                     jump block17
;;
;;                                 block17:
;;                                     jump block11(v7)
;;
;;                                 block11(v5: i32):
;;                                     v57 = load.i64 notrap aligned region5 v17+136
;;                                     v58 = icmp eq v57, v16
;;                                     brif v58, block12, block13
;;
;;                                 block12:
;;                                     v59 = load.i64 notrap aligned region6 v16
;;                                     store notrap aligned region5 v59, v17+136
;;                                     v60 = load.i32 notrap aligned region10 v16+16
;;                                     store notrap aligned region9 v60, v17+128
;;                                     v61 = load.i32 notrap aligned region12 v16+20
;;                                     store notrap aligned region11 v61, v17+132
;;                                     jump block14
;;
;;                                 block13:
;;                                     v65 = load.i64 notrap aligned readonly can_move region23 v2+152
;;                                     v54 = load.i64 notrap aligned readonly can_move region2 v2+168
;;                                     try_call_indirect v65(v54, v2), sig1, block15, [ context v2, default: block8(exn0) ]
;;
;;                                 block15:
;;                                     jump block14
;;
;;                                 block14:
;;                                     store.i32 notrap aligned region4 v12, v11
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
;;                                     jump block22(v5)
;;
;;                                 block22(v9: i32):
;; @0210                               jump block1
;;
;;                                 block1:
;; @0210                               return v9
;; }
