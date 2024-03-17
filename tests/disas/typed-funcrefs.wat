;;! target = "x86_64"
;;! test = "optimize"
;;! flags = [ "-Wfunction-references=y" ]

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
  ;; elements are non-nullable function references, and whose default
  ;; value (needed because non-nullable) is a pointer to `$ic1`.
  (table $ic-sites 100 100 (ref $ic-stub) (ref.func $ic1))

  (func $ic1 (param i32 i32 i32 i32) (result i32)
        local.get 0)

  (func $call-ics (param i32 i32 i32 i32) (result i32)
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

        local.get $sum))
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32) -> i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     sig0 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig1 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;;                                     v6 -> v2
;; @002c                               jump block1
;;
;;                                 block1:
;; @002c                               return v2
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32, i32) -> i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     sig0 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     sig1 = (i64 vmctx, i64, i32, i32, i32, i32) -> i32 fast
;;     sig2 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig3 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;;                                     v21 -> v0
;;                                     v47 -> v0
;;                                     v56 -> v0
;;                                     v59 -> v0
;;                                     v30 -> v2
;;                                     v31 -> v3
;;                                     v32 -> v4
;;                                     v33 -> v5
;;                                     v62 = iconst.i8 0
;; @003b                               brif v62, block6, block7  ; v62 = 0
;;
;;                                 block6 cold:
;; @003b                               trap table_oob
;;
;;                                 block7:
;; @003b                               v12 = load.i64 notrap aligned v0+72
;;                                     v79 = iconst.i8 0
;;                                     v70 = iconst.i64 8
;; @003b                               v14 = iadd v12, v70  ; v70 = 8
;; @003b                               v16 = select_spectre_guard v79, v12, v14  ; v79 = 0
;; @003b                               v17 = load.i64 notrap aligned table v16
;;                                     v58 = iconst.i64 -2
;; @003b                               v18 = band v17, v58  ; v58 = -2
;; @003b                               brif v17, block3(v18), block2
;;
;;                                 block2 cold:
;; @004e                               v48 = load.i64 notrap aligned readonly v0+56
;; @004e                               v49 = load.i64 notrap aligned readonly v48+72
;; @002f                               v7 = iconst.i32 0
;;                                     v28 -> v7
;; @0039                               v8 = iconst.i32 1
;; @003b                               v24 = call_indirect sig0, v49(v0, v7, v8)  ; v7 = 0, v8 = 1
;; @003b                               jump block3(v24)
;;
;;                                 block3(v19: i64):
;; @003d                               brif v19, block9, block8
;;
;;                                 block8 cold:
;; @003d                               trap null_reference
;;
;;                                 block9:
;; @003d                               v25 = load.i64 notrap aligned readonly v19+16
;; @003d                               v26 = load.i64 notrap aligned readonly v19+32
;; @003d                               v27 = call_indirect sig1, v25(v26, v0, v2, v3, v4, v5)
;;                                     v80 = iconst.i8 0
;; @004e                               brif v80, block10, block11  ; v80 = 0
;;
;;                                 block10 cold:
;; @004e                               trap table_oob
;;
;;                                 block11:
;; @004e                               v38 = load.i64 notrap aligned v0+72
;;                                     v81 = iconst.i8 0
;;                                     v78 = iconst.i64 16
;; @004e                               v40 = iadd v38, v78  ; v78 = 16
;; @004e                               v42 = select_spectre_guard v81, v38, v40  ; v81 = 0
;; @004e                               v43 = load.i64 notrap aligned table v42
;;                                     v82 = iconst.i64 -2
;;                                     v83 = band v43, v82  ; v82 = -2
;; @004e                               brif v43, block5(v83), block4
;;
;;                                 block4 cold:
;;                                     v84 = load.i64 notrap aligned readonly v0+56
;;                                     v85 = load.i64 notrap aligned readonly v84+72
;;                                     v86 = iconst.i32 0
;; @004c                               v34 = iconst.i32 2
;; @004e                               v50 = call_indirect sig0, v85(v0, v86, v34)  ; v86 = 0, v34 = 2
;; @004e                               jump block5(v50)
;;
;;                                 block5(v45: i64):
;; @0050                               brif v45, block13, block12
;;
;;                                 block12 cold:
;; @0050                               trap null_reference
;;
;;                                 block13:
;; @0050                               v51 = load.i64 notrap aligned readonly v45+16
;; @0050                               v52 = load.i64 notrap aligned readonly v45+32
;; @0050                               v53 = call_indirect sig1, v51(v52, v0, v2, v3, v4, v5)
;; @0059                               jump block1
;;
;;                                 block1:
;; @0054                               v55 = iadd.i32 v53, v27
;;                                     v6 -> v55
;; @0059                               return v55
;; }
