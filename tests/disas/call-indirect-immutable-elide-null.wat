;;! target = "x86_64"

;; Immutable funcref table where every slot is filled by the elem
;; segment (no "no-entry" gaps). With both the sig check AND the
;; funcref-NULL check elided, the dispatch path is reduced to:
;;   - bounds check (static)
;;   - lazy-init brif + masking
;;   - load code+vmctx
;;   - call_indirect
;;
;; In particular the cold block that handles the runtime trap-on-null
;; path should not exist after the funcref load: the static-match path
;; with `may_be_null = false` skips both the sig check and any
;; downstream null-handling.

(module
  (table 3 3 funcref)

  (func $f1 (result i32) i32.const 1)
  (func $f2 (result i32) i32.const 2)
  (func $f3 (result i32) i32.const 3)

  (func (export "call_it") (param i32) (result i32)
    local.get 0
    call_indirect (result i32))

  ;; Fully cover the table — no null slot anywhere.
  (elem (i32.const 0) func $f1 $f2 $f3))
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
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
;;     region0 = 8 "VMContext+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
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
;;     region0 = 8 "VMContext+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0050                               v4 = iconst.i32 3
;; @0050                               v5 = icmp uge v2, v4  ; v4 = 3
;; @0050                               v6 = uextend.i64 v2
;; @0050                               v7 = load.i64 notrap aligned readonly can_move v0+48
;; @0050                               v8 = iconst.i64 3
;; @0050                               v9 = ishl v6, v8  ; v8 = 3
;; @0050                               v10 = iadd v7, v9
;; @0050                               v11 = iconst.i64 0
;; @0050                               v12 = select_spectre_guard v5, v11, v10  ; v11 = 0
;; @0050                               v13 = load.i64 user6 aligned region1 v12
;; @0050                               v14 = iconst.i64 -2
;; @0050                               v15 = band v13, v14  ; v14 = -2
;; @0050                               brif v13, block3(v15), block2
;;
;;                                 block2 cold:
;; @0050                               v17 = iconst.i32 0
;; @0050                               v18 = uextend.i64 v2
;; @0050                               v19 = call fn0(v0, v17, v18)  ; v17 = 0
;; @0050                               jump block3(v19)
;;
;;                                 block3(v16: i64):
;; @0050                               v20 = load.i64 notrap aligned readonly v16+8
;; @0050                               v21 = load.i64 notrap aligned readonly v16+24
;; @0050                               v22 = call_indirect sig0, v20(v21, v0)
;; @0053                               jump block1
;;
;;                                 block1:
;; @0053                               return v22
;; }
