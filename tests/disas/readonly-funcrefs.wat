;;! target = "x86_64"
;;! test = "optimize"

;; Current WebAssembly toolchains represent source-language function pointers
;; by constructing a single WebAssembly funcref table, initializing it with an
;; element section, and never writing to it again. This test tracks what code
;; we generate for that pattern.

;; Motivated by https://github.com/bytecodealliance/wasmtime/issues/8195

(module
  (type $fn (func))
  (table $fnptrs 2 2 funcref)
  (func $callee)
  (func $caller (param i32)
        local.get 0
        call_indirect $fnptrs (type $fn))
  (elem $fnptrs (i32.const 1) func $callee)
)

;; function u0:0(i64 vmctx, i64) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @002c                               jump block1
;;
;;                                 block1:
;; @002c                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 671088640 "VMTableDefinition+0x0"
;;     region3 = 335544320 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     region4 = 40 "VMContext+0x28"
;;     region5 = 1677721600 "TypeIdsArray+0x0"
;;     region6 = 1610612752 "VMFuncRef+0x10"
;;     region7 = 1610612744 "VMFuncRef+0x8"
;;     region8 = 1610612760 "VMFuncRef+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0031                               v3 = iconst.i32 2
;; @0031                               v4 = icmp uge v2, v3  ; v3 = 2
;; @0031                               v10 = iconst.i64 0
;; @0031                               v6 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0031                               v5 = uextend.i64 v2
;; @0031                               v7 = iconst.i64 3
;; @0031                               v8 = ishl v5, v7  ; v7 = 3
;; @0031                               v9 = iadd v6, v8
;; @0031                               v11 = select_spectre_guard v4, v10, v9  ; v10 = 0
;; @0031                               v12 = load.i64 user6 aligned region3 v11
;; @0031                               v13 = iconst.i64 -2
;; @0031                               v14 = band v12, v13  ; v13 = -2
;; @0031                               brif v12, block3(v14), block2
;;
;;                                 block2 cold:
;; @0031                               v16 = iconst.i32 0
;; @0031                               v18 = call fn0(v0, v16, v5)  ; v16 = 0
;; @0031                               jump block3(v18)
;;
;;                                 block3(v15: i64):
;; @0031                               v21 = load.i32 user7 aligned readonly region6 v15+16
;; @0031                               v19 = load.i64 notrap aligned readonly can_move region4 v0+40
;; @0031                               v20 = load.i32 notrap aligned readonly can_move region5 v19
;; @0031                               v22 = icmp eq v21, v20
;; @0031                               trapz v22, user8
;; @0031                               v24 = load.i64 notrap aligned readonly region7 v15+8
;; @0031                               v25 = load.i64 notrap aligned readonly region8 v15+24
;; @0031                               call_indirect sig0, v24(v25, v0)
;; @0034                               jump block1
;;
;;                                 block1:
;; @0034                               return
;; }
