;;! target = "x86_64"
;;! flags = [ "-Ocache-call-indirects=y" ]

;; This test checks that we do *not* get the indirect-call caching optimization
;; when it must not be used: in this case, because the table is exported so
;; could be mutated (invalidating the cache, which we would not detect).

(module
 (table (export "t") 10 10 funcref)

 (func $f1 (result i32) i32.const 1)
 (func $f2 (result i32) i32.const 2)
 (func $f3 (result i32) i32.const 3)

 (func (export "call_it") (param i32) (result i32)
  local.get 0
  call_indirect (result i32))

 (elem (i32.const 1) func $f1 $f2 $f3))
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0043                               v3 = iconst.i32 1
;; @0045                               jump block1(v3)  ; v3 = 1
;;
;;                                 block1(v2: i32):
;; @0045                               return v2
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0048                               v3 = iconst.i32 2
;; @004a                               jump block1(v3)  ; v3 = 2
;;
;;                                 block1(v2: i32):
;; @004a                               return v2
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @004d                               v3 = iconst.i32 3
;; @004f                               jump block1(v3)  ; v3 = 3
;;
;;                                 block1(v2: i32):
;; @004f                               return v2
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     fn0 = colocated u1:9 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0054                               v4 = global_value.i64 gv3
;; @0054                               v5 = iconst.i32 10
;; @0054                               v6 = icmp uge v2, v5  ; v5 = 10
;; @0054                               v7 = uextend.i64 v2
;; @0054                               v8 = global_value.i64 gv4
;; @0054                               v9 = ishl_imm v7, 3
;; @0054                               v10 = iadd v8, v9
;; @0054                               v11 = iconst.i64 0
;; @0054                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @0054                               v13 = load.i64 table_oob aligned table v12
;; @0054                               v14 = band_imm v13, -2
;; @0054                               brif v13, block3(v14), block2
;;
;;                                 block2 cold:
;; @0054                               v16 = iconst.i32 0
;; @0054                               v17 = global_value.i64 gv3
;; @0054                               v18 = call fn0(v17, v16, v2)  ; v16 = 0
;; @0054                               jump block3(v18)
;;
;;                                 block3(v15: i64):
;; @0054                               v19 = global_value.i64 gv3
;; @0054                               v20 = load.i64 notrap aligned readonly v19+80
;; @0054                               v21 = load.i32 notrap aligned readonly v20
;; @0054                               v22 = load.i32 icall_null aligned readonly v15+16
;; @0054                               v23 = icmp eq v22, v21
;; @0054                               trapz v23, bad_sig
;; @0054                               v24 = load.i64 notrap aligned readonly v15+8
;; @0054                               v25 = load.i64 notrap aligned readonly v15+24
;; @0054                               v26 = call_indirect sig0, v24(v25, v0)
;; @0057                               jump block1(v26)
;;
;;                                 block1(v3: i32):
;; @0057                               return v3
;; }
