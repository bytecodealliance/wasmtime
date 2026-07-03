;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-O opt-level=0"

;; Test basic code generation for table WebAssembly instructions on
;; non-resizeable tables. Use optimized but with `opt-level=0` to legalize away
;; macro instructions.

(module
  (table (export "table") 7 7 externref)
  (func (export "table.get.const") (result externref)
    i32.const 0
    table.get 0)
  (func (export "table.get.var") (param i32) (result externref)
    local.get 0
    table.get 0))

;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 671088640 "VMTableDefinition+0x0"
;;     region3 = 268435456 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0052                               v2 = iconst.i32 0
;; @0054                               v3 = iconst.i32 7
;; @0054                               v4 = icmp uge v2, v3  ; v2 = 0, v3 = 7
;; @0054                               v5 = uextend.i64 v2  ; v2 = 0
;; @0054                               v6 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0054                               v7 = iconst.i64 2
;; @0054                               v8 = ishl v5, v7  ; v7 = 2
;; @0054                               v9 = iadd v6, v8
;; @0054                               v10 = iconst.i64 0
;; @0054                               v11 = select_spectre_guard v4, v10, v9  ; v10 = 0
;; @0054                               v12 = load.i32 user6 aligned region3 v11
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v12
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 671088640 "VMTableDefinition+0x0"
;;     region3 = 268435456 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005b                               v3 = iconst.i32 7
;; @005b                               v4 = icmp uge v2, v3  ; v3 = 7
;; @005b                               v5 = uextend.i64 v2
;; @005b                               v6 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @005b                               v7 = iconst.i64 2
;; @005b                               v8 = ishl v5, v7  ; v7 = 2
;; @005b                               v9 = iadd v6, v8
;; @005b                               v10 = iconst.i64 0
;; @005b                               v11 = select_spectre_guard v4, v10, v9  ; v10 = 0
;; @005b                               v12 = load.i32 user6 aligned region3 v11
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return v12
;; }
