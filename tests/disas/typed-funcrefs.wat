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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i32, i64) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u1:9 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0048                               v12 = load.i64 notrap aligned readonly v0+88
;;                                     v68 = iconst.i64 8
;; @0048                               v14 = iadd v12, v68  ; v68 = 8
;; @0048                               v17 = load.i64 user5 aligned table v14
;;                                     v56 = iconst.i64 -2
;; @0048                               v18 = band v17, v56  ; v56 = -2
;; @0048                               brif v17, block3(v18), block2
;;
;;                                 block2 cold:
;; @003c                               v7 = iconst.i32 0
;;                                     v67 = iconst.i64 1
;; @0048                               v23 = call fn0(v0, v7, v67)  ; v7 = 0, v67 = 1
;; @0048                               jump block3(v23)
;;
;;                                 block3(v19: i64):
;; @004a                               v24 = load.i64 user16 aligned readonly v19+8
;; @004a                               v25 = load.i64 notrap aligned readonly v19+24
;; @004a                               v26 = call_indirect sig1, v24(v25, v0, v2, v3, v4, v5)
;;                                     v76 = iconst.i64 16
;; @005b                               v39 = iadd.i64 v12, v76  ; v76 = 16
;; @005b                               v42 = load.i64 user5 aligned table v39
;;                                     v77 = iconst.i64 -2
;;                                     v78 = band v42, v77  ; v77 = -2
;; @005b                               brif v42, block5(v78), block4
;;
;;                                 block4 cold:
;;                                     v79 = iconst.i32 0
;;                                     v75 = iconst.i64 2
;; @005b                               v48 = call fn0(v0, v79, v75)  ; v79 = 0, v75 = 2
;; @005b                               jump block5(v48)
;;
;;                                 block5(v44: i64):
;; @005d                               v49 = load.i64 user16 aligned readonly v44+8
;; @005d                               v50 = load.i64 notrap aligned readonly v44+24
;; @005d                               v51 = call_indirect sig1, v49(v50, v0, v2, v3, v4, v5)
;; @0066                               jump block1
;;
;;                                 block1:
;; @0061                               v53 = iadd.i32 v51, v26
;; @0066                               return v53
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u1:9 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0075                               v12 = load.i64 notrap aligned readonly v0+88
;;                                     v68 = iconst.i64 8
;; @0075                               v14 = iadd v12, v68  ; v68 = 8
;; @0075                               v17 = load.i64 user5 aligned table v14
;;                                     v56 = iconst.i64 -2
;; @0075                               v18 = band v17, v56  ; v56 = -2
;; @0075                               brif v17, block3(v18), block2
;;
;;                                 block2 cold:
;; @0069                               v7 = iconst.i32 0
;;                                     v67 = iconst.i64 1
;; @0075                               v23 = call fn0(v0, v7, v67)  ; v7 = 0, v67 = 1
;; @0075                               jump block3(v23)
;;
;;                                 block3(v19: i64):
;; @0075                               v24 = load.i64 user6 aligned readonly v19+8
;; @0075                               v25 = load.i64 notrap aligned readonly v19+24
;; @0075                               v26 = call_indirect sig0, v24(v25, v0, v2, v3, v4, v5)
;;                                     v76 = iconst.i64 16
;; @0087                               v39 = iadd.i64 v12, v76  ; v76 = 16
;; @0087                               v42 = load.i64 user5 aligned table v39
;;                                     v77 = iconst.i64 -2
;;                                     v78 = band v42, v77  ; v77 = -2
;; @0087                               brif v42, block5(v78), block4
;;
;;                                 block4 cold:
;;                                     v79 = iconst.i32 0
;;                                     v75 = iconst.i64 2
;; @0087                               v48 = call fn0(v0, v79, v75)  ; v79 = 0, v75 = 2
;; @0087                               jump block5(v48)
;;
;;                                 block5(v44: i64):
;; @0087                               v49 = load.i64 user6 aligned readonly v44+8
;; @0087                               v50 = load.i64 notrap aligned readonly v44+24
;; @0087                               v51 = call_indirect sig0, v49(v50, v0, v2, v3, v4, v5)
;; @0091                               jump block1
;;
;;                                 block1:
;; @008c                               v53 = iadd.i32 v51, v26
;; @0091                               return v53
;; }
;;
;; function u0:3(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @009e                               v9 = load.i64 notrap aligned table v0+112
;; @00a0                               v10 = load.i64 user16 aligned readonly v9+8
;; @00a0                               v11 = load.i64 notrap aligned readonly v9+24
;; @00a0                               v12 = call_indirect sig0, v10(v11, v0, v2, v3, v4, v5)
;; @00af                               v15 = load.i64 notrap aligned table v0+128
;; @00b1                               v16 = load.i64 user16 aligned readonly v15+8
;; @00b1                               v17 = load.i64 notrap aligned readonly v15+24
;; @00b1                               v18 = call_indirect sig0, v16(v17, v0, v2, v3, v4, v5)
;; @00ba                               jump block1
;;
;;                                 block1:
;; @00b5                               v19 = iadd.i32 v18, v12
;; @00ba                               return v19
;; }
