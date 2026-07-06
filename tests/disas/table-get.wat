;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-O opt-level=0"

;; Test basic code generation for table WebAssembly instructions.
;; Use optimization but with `opt-level=0` to legalize away table_addr instructions.

(module
  (table (export "table") 1 externref)
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
;;     region3 = 671088648 "VMTableDefinition+0x8"
;;     region4 = 268435456 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0051                               v2 = iconst.i32 0
;; @0053                               v3 = load.i64 notrap aligned region3 v0+56
;; @0053                               v4 = ireduce.i32 v3
;; @0053                               v5 = icmp uge v2, v4  ; v2 = 0
;; @0053                               v6 = uextend.i64 v2  ; v2 = 0
;; @0053                               v7 = load.i64 notrap aligned region2 v0+48
;; @0053                               v8 = iconst.i64 2
;; @0053                               v9 = ishl v6, v8  ; v8 = 2
;; @0053                               v10 = iadd v7, v9
;; @0053                               v11 = iconst.i64 0
;; @0053                               v12 = select_spectre_guard v5, v11, v10  ; v11 = 0
;; @0053                               v13 = load.i32 user6 aligned region4 v12
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return v13
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 671088640 "VMTableDefinition+0x0"
;;     region3 = 671088648 "VMTableDefinition+0x8"
;;     region4 = 268435456 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005a                               v3 = load.i64 notrap aligned region3 v0+56
;; @005a                               v4 = ireduce.i32 v3
;; @005a                               v5 = icmp uge v2, v4
;; @005a                               v6 = uextend.i64 v2
;; @005a                               v7 = load.i64 notrap aligned region2 v0+48
;; @005a                               v8 = iconst.i64 2
;; @005a                               v9 = ishl v6, v8  ; v8 = 2
;; @005a                               v10 = iadd v7, v9
;; @005a                               v11 = iconst.i64 0
;; @005a                               v12 = select_spectre_guard v5, v11, v10  ; v11 = 0
;; @005a                               v13 = load.i32 user6 aligned region4 v12
;; @005c                               jump block1
;;
;;                                 block1:
;; @005c                               return v13
;; }
