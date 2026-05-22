;;! target = "x86_64"

;; Table declared with min < max (a "dynamic-declared" table) that is
;; never written to in the module. Without the per-table mutability
;; bit, Cranelift would emit `load.i64 v0+56` per dispatch to fetch
;; the current bound. With it, `make_table` lowers to
;; `TableSize::Static` and the bound becomes an immediate.
;;
;; Look for: bounds-check `iconst.i32 16` (the declared min, used as
;; static bound) and NO `load.i64 ... v0+56` for the current_elements
;; field. (`+48` for the funcref base is still loaded — that's the
;; element-data pointer, separate from the bound.)

(module
  ;; min=16, max=64 — distinct, so without our optimization the
  ;; bound would be loaded per dispatch from `current_elements`.
  (table 16 64 funcref)

  (func $f1 (result i32) i32.const 1)
  (func $f2 (result i32) i32.const 2)
  (func $f3 (result i32) i32.const 3)

  (func (export "call_it") (param i32) (result i32)
    local.get 0
    call_indirect (result i32))

  (elem (i32.const 0) func $f1 $f2 $f3))
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003f                               v3 = iconst.i32 1
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return v3  ; v3 = 1
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0044                               v3 = iconst.i32 2
;; @0046                               jump block1
;;
;;                                 block1:
;; @0046                               return v3  ; v3 = 2
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0049                               v3 = iconst.i32 3
;; @004b                               jump block1
;;
;;                                 block1:
;; @004b                               return v3  ; v3 = 3
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:9 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0050                               v4 = iconst.i32 16
;; @0050                               v5 = icmp uge v2, v4  ; v4 = 16
;; @0050                               v6 = uextend.i64 v2
;; @0050                               v7 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v23 = iconst.i64 3
;; @0050                               v8 = ishl v6, v23  ; v23 = 3
;; @0050                               v9 = iadd v7, v8
;; @0050                               v10 = iconst.i64 0
;; @0050                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0050                               v12 = load.i64 user5 aligned table v11
;;                                     v22 = iconst.i64 -2
;; @0050                               v13 = band v12, v22  ; v22 = -2
;; @0050                               brif v12, block3(v13), block2
;;
;;                                 block2 cold:
;; @0050                               v15 = iconst.i32 0
;; @0050                               v17 = uextend.i64 v2
;; @0050                               v18 = call fn0(v0, v15, v17)  ; v15 = 0
;; @0050                               jump block3(v18)
;;
;;                                 block3(v14: i64):
;; @0050                               v19 = load.i64 user6 aligned readonly v14+8
;; @0050                               v20 = load.i64 notrap aligned readonly v14+24
;; @0050                               v21 = call_indirect sig0, v19(v20, v0)
;; @0053                               jump block1
;;
;;                                 block1:
;; @0053                               return v21
;; }
