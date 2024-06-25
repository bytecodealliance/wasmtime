;;! target = "x86_64"
;;! test = "optimize"
;;! flags = [ "-Wfunction-references=y", "-Otable-lazy-init=n" ]

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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0048                               v12 = load.i64 notrap aligned readonly v0+88
;;                                     v48 = iconst.i64 8
;; @0048                               v14 = iadd v12, v48  ; v48 = 8
;; @0048                               v17 = load.i64 table_oob aligned table v14
;; @004a                               v18 = load.i64 null_reference aligned readonly v17+8
;; @004a                               v19 = load.i64 notrap aligned readonly v17+24
;; @004a                               v20 = call_indirect sig0, v18(v19, v0, v2, v3, v4, v5)
;;                                     v56 = iconst.i64 16
;; @005b                               v28 = iadd v12, v56  ; v56 = 16
;; @005b                               v31 = load.i64 table_oob aligned table v28
;; @005d                               v32 = load.i64 null_reference aligned readonly v31+8
;; @005d                               v33 = load.i64 notrap aligned readonly v31+24
;; @005d                               v34 = call_indirect sig0, v32(v33, v0, v2, v3, v4, v5)
;; @0066                               jump block1
;;
;;                                 block1:
;; @0061                               v35 = iadd.i32 v34, v20
;; @0066                               return v35
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0075                               v13 = load.i64 notrap aligned readonly v0+88
;;                                     v50 = iconst.i64 8
;; @0075                               v15 = iadd v13, v50  ; v50 = 8
;; @0075                               v18 = load.i64 table_oob aligned table v15
;; @0075                               v19 = load.i64 icall_null aligned readonly v18+8
;; @0075                               v20 = load.i64 notrap aligned readonly v18+24
;; @0075                               v21 = call_indirect sig0, v19(v20, v0, v2, v3, v4, v5)
;;                                     v58 = iconst.i64 16
;; @0087                               v30 = iadd v13, v58  ; v58 = 16
;; @0087                               v33 = load.i64 table_oob aligned table v30
;; @0087                               v34 = load.i64 icall_null aligned readonly v33+8
;; @0087                               v35 = load.i64 notrap aligned readonly v33+24
;; @0087                               v36 = call_indirect sig0, v34(v35, v0, v2, v3, v4, v5)
;; @0091                               jump block1
;;
;;                                 block1:
;; @008c                               v37 = iadd.i32 v36, v21
;; @0091                               return v37
;; }
;;
;; function u0:3(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @009e                               v9 = load.i64 notrap aligned table v0+112
;; @00a0                               v10 = load.i64 null_reference aligned readonly v9+8
;; @00a0                               v11 = load.i64 notrap aligned readonly v9+24
;; @00a0                               v12 = call_indirect sig0, v10(v11, v0, v2, v3, v4, v5)
;; @00af                               v15 = load.i64 notrap aligned table v0+128
;; @00b1                               v16 = load.i64 null_reference aligned readonly v15+8
;; @00b1                               v17 = load.i64 notrap aligned readonly v15+24
;; @00b1                               v18 = call_indirect sig0, v16(v17, v0, v2, v3, v4, v5)
;; @00ba                               jump block1
;;
;;                                 block1:
;; @00b5                               v19 = iadd.i32 v18, v12
;; @00ba                               return v19
;; }
