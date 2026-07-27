;;! target = "x86_64"

;; Immutable funcref table where every elem-segment entry has the same
;; declared type as the call site. This module's `tables_mutated` bit
;; for table 0 is clear (no opcode in any function writes to it), and
;; all three slots resolve to the same module type as the call site.
;; That triggers `try_elide_sig_check_for_immutable_table` →
;; `CheckIndirectCallTypeSignature::StaticMatch`, removing the runtime
;; signature load + compare from the dispatch hot path.
;;
;; Look for the absence of `load.i32 user6 aligned readonly v_+16` (the
;; sig-id load) and the matching `icmp eq / trapz user7` on the call
;; site. Compare with `indirect-call-no-caching.wat` for the
;; non-elided shape.

(module
  (table 10 10 funcref)

  (func $f1 (result i32) i32.const 1)
  (func $f2 (result i32) i32.const 2)
  (func $f3 (result i32) i32.const 3)

  (func (export "call_it") (param i32) (result i32)
    local.get 0
    call_indirect (result i32))

  (elem (i32.const 0) func $f1 $f2 $f3))
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003f                               v2 = iconst.i32 1
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return v2  ; v2 = 1
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0044                               v2 = iconst.i32 2
;; @0046                               jump block1
;;
;;                                 block1:
;; @0046                               return v2  ; v2 = 2
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0049                               v2 = iconst.i32 3
;; @004b                               jump block1
;;
;;                                 block1:
;; @004b                               return v2  ; v2 = 3
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 671088640 "VMTableDefinition+0x0"
;;     region3 = 335544320 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     region4 = 1610612744 "VMFuncRef+0x8"
;;     region5 = 1610612760 "VMFuncRef+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0050                               v3 = iconst.i32 10
;; @0050                               v4 = icmp uge v2, v3  ; v3 = 10
;; @0050                               v5 = uextend.i64 v2
;; @0050                               v6 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0050                               v7 = iconst.i64 3
;; @0050                               v8 = ishl v5, v7  ; v7 = 3
;; @0050                               v9 = iadd v6, v8
;; @0050                               v10 = iconst.i64 0
;; @0050                               v11 = select_spectre_guard v4, v10, v9  ; v10 = 0
;; @0050                               v12 = load.i64 user6 aligned region3 v11
;; @0050                               v13 = iconst.i64 -2
;; @0050                               v14 = band v12, v13  ; v13 = -2
;; @0050                               brif v12, block3(v14), block2
;;
;;                                 block2 cold:
;; @0050                               v16 = iconst.i32 0
;; @0050                               v17 = uextend.i64 v2
;; @0050                               v18 = call fn0(v0, v16, v17)  ; v16 = 0
;; @0050                               jump block3(v18)
;;
;;                                 block3(v15: i64):
;; @0050                               v19 = load.i64 user7 aligned readonly region4 v15+8
;; @0050                               v20 = load.i64 notrap aligned readonly region5 v15+24
;; @0050                               v21 = call_indirect sig0, v19(v20, v0)
;; @0053                               jump block1
;;
;;                                 block1:
;; @0053                               return v21
;; }
