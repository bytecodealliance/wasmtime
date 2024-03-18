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
;; @0039                               jump block1
;;
;;                                 block1:
;; @0039                               return v2
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
;; @0048                               brif v62, block6, block7  ; v62 = 0
;;
;;                                 block6 cold:
;; @0048                               trap table_oob
;;
;;                                 block7:
;; @0048                               v12 = load.i64 notrap aligned v0+72
;;                                     v79 = iconst.i8 0
;;                                     v70 = iconst.i64 8
;; @0048                               v14 = iadd v12, v70  ; v70 = 8
;; @0048                               v16 = select_spectre_guard v79, v12, v14  ; v79 = 0
;; @0048                               v17 = load.i64 notrap aligned table v16
;;                                     v58 = iconst.i64 -2
;; @0048                               v18 = band v17, v58  ; v58 = -2
;; @0048                               brif v17, block3(v18), block2
;;
;;                                 block2 cold:
;; @005b                               v48 = load.i64 notrap aligned readonly v0+56
;; @005b                               v49 = load.i64 notrap aligned readonly v48+72
;; @003c                               v7 = iconst.i32 0
;;                                     v28 -> v7
;; @0046                               v8 = iconst.i32 1
;; @0048                               v24 = call_indirect sig0, v49(v0, v7, v8)  ; v7 = 0, v8 = 1
;; @0048                               jump block3(v24)
;;
;;                                 block3(v19: i64):
;; @004a                               v25 = load.i64 null_reference aligned readonly v19+16
;; @004a                               v26 = load.i64 notrap aligned readonly v19+32
;; @004a                               v27 = call_indirect sig1, v25(v26, v0, v2, v3, v4, v5)
;;                                     v80 = iconst.i8 0
;; @005b                               brif v80, block8, block9  ; v80 = 0
;;
;;                                 block8 cold:
;; @005b                               trap table_oob
;;
;;                                 block9:
;; @005b                               v38 = load.i64 notrap aligned v0+72
;;                                     v81 = iconst.i8 0
;;                                     v78 = iconst.i64 16
;; @005b                               v40 = iadd v38, v78  ; v78 = 16
;; @005b                               v42 = select_spectre_guard v81, v38, v40  ; v81 = 0
;; @005b                               v43 = load.i64 notrap aligned table v42
;;                                     v82 = iconst.i64 -2
;;                                     v83 = band v43, v82  ; v82 = -2
;; @005b                               brif v43, block5(v83), block4
;;
;;                                 block4 cold:
;;                                     v84 = load.i64 notrap aligned readonly v0+56
;;                                     v85 = load.i64 notrap aligned readonly v84+72
;;                                     v86 = iconst.i32 0
;; @0059                               v34 = iconst.i32 2
;; @005b                               v50 = call_indirect sig0, v85(v0, v86, v34)  ; v86 = 0, v34 = 2
;; @005b                               jump block5(v50)
;;
;;                                 block5(v45: i64):
;; @005d                               v51 = load.i64 null_reference aligned readonly v45+16
;; @005d                               v52 = load.i64 notrap aligned readonly v45+32
;; @005d                               v53 = call_indirect sig1, v51(v52, v0, v2, v3, v4, v5)
;; @0066                               jump block1
;;
;;                                 block1:
;; @0061                               v55 = iadd.i32 v53, v27
;;                                     v6 -> v55
;; @0066                               return v55
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32, i32) -> i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     sig0 = (i64 vmctx, i64, i32, i32, i32, i32) -> i32 fast
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     sig2 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig3 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;;                                     v21 -> v0
;;                                     v25 -> v0
;;                                     v52 -> v0
;;                                     v56 -> v0
;;                                     v66 -> v0
;;                                     v69 -> v0
;;                                     v35 -> v2
;;                                     v36 -> v3
;;                                     v37 -> v4
;;                                     v38 -> v5
;;                                     v72 = iconst.i8 0
;; @0075                               brif v72, block6, block7  ; v72 = 0
;;
;;                                 block6 cold:
;; @0075                               trap table_oob
;;
;;                                 block7:
;; @0075                               v12 = load.i64 notrap aligned v0+72
;;                                     v89 = iconst.i8 0
;;                                     v80 = iconst.i64 8
;; @0075                               v14 = iadd v12, v80  ; v80 = 8
;; @0075                               v16 = select_spectre_guard v89, v12, v14  ; v89 = 0
;; @0075                               v17 = load.i64 notrap aligned table v16
;;                                     v68 = iconst.i64 -2
;; @0075                               v18 = band v17, v68  ; v68 = -2
;; @0075                               brif v17, block3(v18), block2
;;
;;                                 block2 cold:
;; @0087                               v53 = load.i64 notrap aligned readonly v0+56
;; @0087                               v54 = load.i64 notrap aligned readonly v53+72
;; @0069                               v7 = iconst.i32 0
;;                                     v33 -> v7
;; @0073                               v8 = iconst.i32 1
;; @0075                               v24 = call_indirect sig1, v54(v0, v7, v8)  ; v7 = 0, v8 = 1
;; @0075                               jump block3(v24)
;;
;;                                 block3(v19: i64):
;; @0075                               v28 = load.i32 icall_null aligned readonly v19+24
;; @0075                               v26 = load.i64 notrap aligned readonly v0+64
;; @0075                               v27 = load.i32 notrap aligned readonly v26
;; @0075                               v29 = icmp eq v28, v27
;; @0075                               brif v29, block9, block8
;;
;;                                 block8 cold:
;; @0075                               trap bad_sig
;;
;;                                 block9:
;; @0075                               v30 = load.i64 notrap aligned readonly v19+16
;; @0075                               v31 = load.i64 notrap aligned readonly v19+32
;; @0075                               v32 = call_indirect sig0, v30(v31, v0, v2, v3, v4, v5)
;;                                     v90 = iconst.i8 0
;; @0087                               brif v90, block10, block11  ; v90 = 0
;;
;;                                 block10 cold:
;; @0087                               trap table_oob
;;
;;                                 block11:
;; @0087                               v43 = load.i64 notrap aligned v0+72
;;                                     v91 = iconst.i8 0
;;                                     v88 = iconst.i64 16
;; @0087                               v45 = iadd v43, v88  ; v88 = 16
;; @0087                               v47 = select_spectre_guard v91, v43, v45  ; v91 = 0
;; @0087                               v48 = load.i64 notrap aligned table v47
;;                                     v92 = iconst.i64 -2
;;                                     v93 = band v48, v92  ; v92 = -2
;; @0087                               brif v48, block5(v93), block4
;;
;;                                 block4 cold:
;;                                     v94 = load.i64 notrap aligned readonly v0+56
;;                                     v95 = load.i64 notrap aligned readonly v94+72
;;                                     v96 = iconst.i32 0
;; @0085                               v39 = iconst.i32 2
;; @0087                               v55 = call_indirect sig1, v95(v0, v96, v39)  ; v96 = 0, v39 = 2
;; @0087                               jump block5(v55)
;;
;;                                 block5(v50: i64):
;; @0087                               v59 = load.i32 icall_null aligned readonly v50+24
;; @0087                               v60 = icmp eq v59, v27
;; @0087                               brif v60, block13, block12
;;
;;                                 block12 cold:
;; @0087                               trap bad_sig
;;
;;                                 block13:
;; @0087                               v61 = load.i64 notrap aligned readonly v50+16
;; @0087                               v62 = load.i64 notrap aligned readonly v50+32
;; @0087                               v63 = call_indirect sig0, v61(v62, v0, v2, v3, v4, v5)
;; @0091                               jump block1
;;
;;                                 block1:
;; @008c                               v65 = iadd.i32 v63, v32
;;                                     v6 -> v65
;; @0091                               return v65
;; }
;;
;; function u0:3(i64 vmctx, i64, i32, i32, i32, i32) -> i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i64, i32, i32, i32, i32) -> i32 fast
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig2 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;;                                     v8 -> v0
;;                                     v14 -> v0
;; @009e                               v9 = load.i64 notrap aligned table v0+96
;; @00a0                               v10 = load.i64 null_reference aligned readonly v9+16
;; @00a0                               v11 = load.i64 notrap aligned readonly v9+32
;; @00a0                               v12 = call_indirect sig0, v10(v11, v0, v2, v3, v4, v5)
;; @00af                               v15 = load.i64 notrap aligned table v0+112
;; @00b1                               v16 = load.i64 null_reference aligned readonly v15+16
;; @00b1                               v17 = load.i64 notrap aligned readonly v15+32
;; @00b1                               v18 = call_indirect sig0, v16(v17, v0, v2, v3, v4, v5)
;; @00ba                               jump block1
;;
;;                                 block1:
;; @00b5                               v19 = iadd.i32 v18, v12
;;                                     v6 -> v19
;; @00ba                               return v19
;; }
