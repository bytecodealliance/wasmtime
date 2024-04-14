;;! target = "x86_64"
;;! flags = [ "-Wcache-call-indirects=y" ]

(module
 (table 10 10 funcref)

 (func $f1 (result i32) i32.const 1)
 (func $f2 (result i32) i32.const 2)
 (func $f3 (result i32) i32.const 3)

 (func (export "call_it") (param i32) (result i32)
  local.get 0
  call_indirect (result i32))

 (func (export "update_table")
  i32.const 1
  ref.null func
  table.set)

 (elem (i32.const 1) func $f1 $f2 $f3))
;; function u0:0(i64 vmctx, i64) -> i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0052                               v3 = iconst.i32 1
;; @0054                               jump block1(v3)  ; v3 = 1
;;
;;                                 block1(v2: i32):
;; @0054                               return v2
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0057                               v3 = iconst.i32 2
;; @0059                               jump block1(v3)  ; v3 = 2
;;
;;                                 block1(v2: i32):
;; @0059                               return v2
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @005c                               v3 = iconst.i32 3
;; @005e                               jump block1(v3)  ; v3 = 3
;;
;;                                 block1(v2: i32):
;; @005e                               return v2
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i64) -> i32 fast
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     fn0 = colocated u1:9 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0063                               v4 = iconst.i32 10
;; @0063                               v5 = icmp uge v2, v4  ; v4 = 10
;; @0063                               v6 = uextend.i64 v2
;; @0063                               v7 = global_value.i64 gv4
;; @0063                               v8 = ishl_imm v6, 3
;; @0063                               v9 = iadd v7, v8
;; @0063                               v10 = iconst.i64 0
;; @0063                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0063                               v12 = load.i64 table_oob aligned table v11
;; @0063                               v13 = band_imm v12, -2
;; @0063                               brif v12, block3(v13), block2
;;
;;                                 block2 cold:
;; @0063                               v15 = iconst.i32 0
;; @0063                               v16 = global_value.i64 gv3
;; @0063                               v17 = call fn0(v16, v15, v2)  ; v15 = 0
;; @0063                               jump block3(v17)
;;
;;                                 block3(v14: i64):
;; @0063                               v18 = global_value.i64 gv3
;; @0063                               v19 = load.i64 notrap aligned readonly v18+80
;; @0063                               v20 = load.i32 notrap aligned readonly v19
;; @0063                               v21 = load.i32 icall_null aligned readonly v14+24
;; @0063                               v22 = icmp eq v21, v20
;; @0063                               trapz v22, bad_sig
;; @0063                               v23 = load.i64 notrap aligned readonly v14+16
;; @0063                               v24 = load.i64 notrap aligned readonly v14+32
;; @0063                               v25 = call_indirect sig0, v23(v24, v0)
;; @0066                               jump block1(v25)
;;
;;                                 block1(v3: i32):
;; @0066                               return v3
;; }
;;
;; function u0:4(i64 vmctx, i64) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0069                               v2 = iconst.i32 1
;; @006b                               v3 = iconst.i64 0
;; @006d                               v4 = iconst.i32 10
;; @006d                               v5 = icmp uge v2, v4  ; v2 = 1, v4 = 10
;; @006d                               v6 = uextend.i64 v2  ; v2 = 1
;; @006d                               v7 = global_value.i64 gv4
;; @006d                               v8 = ishl_imm v6, 3
;; @006d                               v9 = iadd v7, v8
;; @006d                               v10 = iconst.i64 0
;; @006d                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @006d                               v12 = bor_imm v3, 1  ; v3 = 0
;; @006d                               store table_oob aligned table v12, v11
;; @006f                               jump block1
;;
;;                                 block1:
;; @006f                               return
;; }
