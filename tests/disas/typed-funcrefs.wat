;;! target = "x86_64"
;;! test = "optimize"
;;! flags = [ "-Wfunction-references=y", "-Otable-lazy-init=y" ]

;; This test is meant to simulate how typed funcrefs in a table may be
;; used for ICs (inline caches) in a Wasm module compiled from a dynamic
;; language. In native JIT engines, IC chains have head pointers that
;; are raw code pointers and IC-using code can call each with a few ops
;; (load pointer, call indirect). We'd like similar efficiency by
;; storing funcrefs for the first IC in each chain in a typed-funcref
;; table.

(module
  (type $ic-stub (func (param i32 i32 i32 i32) (result i32)))

  ;; This syntax declares a table that is exactly 100 elements, whose
  ;; elements are nullable function references, and whose default
  ;; value is `null`.
  (table $ic-sites 100 100 (ref null $ic-stub))

  (func $ic1 (param i32 i32 i32 i32) (result i32)
        local.get 0)

  ;; A function which uses ICs through `table.get` plus `call_ref`
  (func $call-ics-with-call-ref (param i32 i32 i32 i32) (result i32)
        (local $sum i32)

        ;; IC callsite index 1 (arbitrary).
        local.get 0
        local.get 1
        local.get 2
        local.get 3
        i32.const 1
        table.get $ic-sites
        call_ref $ic-stub
        local.get $sum
        i32.add
        local.set $sum

        ;; IC callsite index 2 (arbitrary).
        local.get 0
        local.get 1
        local.get 2
        local.get 3
        i32.const 2
        table.get $ic-sites
        call_ref $ic-stub
        local.get $sum
        i32.add
        local.set $sum

        local.get $sum)

  ;; Same as the above function, but uses `call_indirect` rather than
  ;; `call_ref`.
  (func $call-ics-with-call-indirect (param i32 i32 i32 i32) (result i32)
        (local $sum i32)

        ;; IC callsite index 1 (arbitrary).
        local.get 0
        local.get 1
        local.get 2
        local.get 3
        i32.const 1
        call_indirect $ic-sites (type $ic-stub)
        local.get $sum
        i32.add
        local.set $sum

        ;; IC callsite index 2 (arbitrary).
        local.get 0
        local.get 1
        local.get 2
        local.get 3
        i32.const 2
        call_indirect $ic-sites (type $ic-stub)
        local.get $sum
        i32.add
        local.set $sum

        local.get $sum)

  (global $ic-site0 (mut (ref $ic-stub)) (ref.func $ic1))
  (global $ic-site1 (mut (ref $ic-stub)) (ref.func $ic1))

  ;; Sort of similar to the previous two functions, but uses globals instead of
  ;; tables to store ICs. Mostly just here for comparison in terms of codegen.
  (func $call-ics-with-global-get (param i32 i32 i32 i32) (result i32)
        (local $sum i32)

        ;; IC callsite index 1 (arbitrary).
        local.get 0
        local.get 1
        local.get 2
        local.get 3
        global.get $ic-site0
        call_ref $ic-stub
        local.get $sum
        i32.add
        local.set $sum

        ;; IC callsite index 2 (arbitrary).
        local.get 0
        local.get 1
        local.get 2
        local.get 3
        global.get $ic-site1
        call_ref $ic-stub
        local.get $sum
        i32.add
        local.set $sum

        local.get $sum)
)

;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0039                               jump block1
;;
;;                                 block1:
;; @0039                               return v2
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2684354560 "VMTableDefinition+0x0"
;;     region3 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i64) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:7 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0048                               v11 = load.i64 notrap aligned readonly can_move region2 v0+48
;;                                     v62 = iconst.i64 8
;; @0048                               v14 = iadd v11, v62  ; v62 = 8
;; @0048                               v17 = load.i64 user6 aligned region3 v14
;; @0048                               v18 = iconst.i64 -2
;; @0048                               v19 = band v17, v18  ; v18 = -2
;; @0048                               brif v17, block3(v19), block2
;;
;;                                 block2 cold:
;; @003c                               v6 = iconst.i32 0
;;                                     v61 = iconst.i64 1
;; @0048                               v23 = call fn0(v0, v6, v61)  ; v6 = 0, v61 = 1
;; @0048                               jump block3(v23)
;;
;;                                 block3(v20: i64):
;; @004a                               v24 = load.i64 user16 aligned readonly v20+8
;; @004a                               v25 = load.i64 notrap aligned readonly v20+24
;; @004a                               v26 = call_indirect sig1, v24(v25, v0, v2, v3, v4, v5)
;;                                     v69 = iconst.i64 16
;; @005b                               v40 = iadd.i64 v11, v69  ; v69 = 16
;; @005b                               v43 = load.i64 user6 aligned region3 v40
;;                                     v70 = iconst.i64 -2
;;                                     v71 = band v43, v70  ; v70 = -2
;; @005b                               brif v43, block5(v71), block4
;;
;;                                 block4 cold:
;;                                     v72 = iconst.i32 0
;;                                     v68 = iconst.i64 2
;; @005b                               v49 = call fn0(v0, v72, v68)  ; v72 = 0, v68 = 2
;; @005b                               jump block5(v49)
;;
;;                                 block5(v46: i64):
;; @005d                               v50 = load.i64 user16 aligned readonly v46+8
;; @005d                               v51 = load.i64 notrap aligned readonly v46+24
;; @005d                               v52 = call_indirect sig1, v50(v51, v0, v2, v3, v4, v5)
;; @0066                               jump block1
;;
;;                                 block1:
;; @0061                               v54 = iadd.i32 v52, v26
;; @0066                               return v54
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2684354560 "VMTableDefinition+0x0"
;;     region3 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0075                               v11 = load.i64 notrap aligned readonly can_move region2 v0+48
;;                                     v62 = iconst.i64 8
;; @0075                               v14 = iadd v11, v62  ; v62 = 8
;; @0075                               v17 = load.i64 user6 aligned region3 v14
;; @0075                               v18 = iconst.i64 -2
;; @0075                               v19 = band v17, v18  ; v18 = -2
;; @0075                               brif v17, block3(v19), block2
;;
;;                                 block2 cold:
;; @0069                               v6 = iconst.i32 0
;;                                     v61 = iconst.i64 1
;; @0075                               v23 = call fn0(v0, v6, v61)  ; v6 = 0, v61 = 1
;; @0075                               jump block3(v23)
;;
;;                                 block3(v20: i64):
;; @0075                               v24 = load.i64 user7 aligned readonly v20+8
;; @0075                               v25 = load.i64 notrap aligned readonly v20+24
;; @0075                               v26 = call_indirect sig0, v24(v25, v0, v2, v3, v4, v5)
;;                                     v69 = iconst.i64 16
;; @0087                               v40 = iadd.i64 v11, v69  ; v69 = 16
;; @0087                               v43 = load.i64 user6 aligned region3 v40
;;                                     v70 = iconst.i64 -2
;;                                     v71 = band v43, v70  ; v70 = -2
;; @0087                               brif v43, block5(v71), block4
;;
;;                                 block4 cold:
;;                                     v72 = iconst.i32 0
;;                                     v68 = iconst.i64 2
;; @0087                               v49 = call fn0(v0, v72, v68)  ; v72 = 0, v68 = 2
;; @0087                               jump block5(v49)
;;
;;                                 block5(v46: i64):
;; @0087                               v50 = load.i64 user7 aligned readonly v46+8
;; @0087                               v51 = load.i64 notrap aligned readonly v46+24
;; @0087                               v52 = call_indirect sig0, v50(v51, v0, v2, v3, v4, v5)
;; @0091                               jump block1
;;
;;                                 block1:
;; @008c                               v54 = iadd.i32 v52, v26
;; @0091                               return v54
;; }
;;
;; function u0:3(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 1879048192 "DefinedGlobal(StaticModuleIndex(0), DefinedGlobalIndex(0))"
;;     region3 = 1879048193 "DefinedGlobal(StaticModuleIndex(0), DefinedGlobalIndex(1))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @009e                               v7 = load.i64 notrap aligned region2 v0+64
;; @00a0                               v8 = load.i64 user16 aligned readonly v7+8
;; @00a0                               v9 = load.i64 notrap aligned readonly v7+24
;; @00a0                               v10 = call_indirect sig0, v8(v9, v0, v2, v3, v4, v5)
;; @00af                               v12 = load.i64 notrap aligned region3 v0+80
;; @00b1                               v13 = load.i64 user16 aligned readonly v12+8
;; @00b1                               v14 = load.i64 notrap aligned readonly v12+24
;; @00b1                               v15 = call_indirect sig0, v13(v14, v0, v2, v3, v4, v5)
;; @00ba                               jump block1
;;
;;                                 block1:
;; @00b5                               v16 = iadd.i32 v15, v10
;; @00ba                               return v16
;; }
