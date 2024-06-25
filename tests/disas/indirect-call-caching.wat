;;! target = "x86_64"
;;! flags = [ "-Ocache-call-indirects=y" ]

;; This test checks that we get the indirect-call caching optimization
;; where it should be applicable (immutable table, null 0-index).
;;
;; The key bit in the expectation below is the cached-index load (v6),
;; compare (v7), branch, fastpath in block2/block4.

(module
 (table 10 10 funcref)

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
;; @003f                               v3 = iconst.i32 1
;; @0041                               jump block1(v3)  ; v3 = 1
;;
;;                                 block1(v2: i32):
;; @0041                               return v2
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0044                               v3 = iconst.i32 2
;; @0046                               jump block1(v3)  ; v3 = 2
;;
;;                                 block1(v2: i32):
;; @0046                               return v2
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0049                               v3 = iconst.i32 3
;; @004b                               jump block1(v3)  ; v3 = 3
;;
;;                                 block1(v2: i32):
;; @004b                               return v2
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
;; @0050                               v4 = global_value.i64 gv3
;; @0050                               v5 = band_imm v2, 1023
;; @0050                               v6 = iadd_imm v5, 0
;; @0050                               v7 = iadd_imm v4, 240
;; @0050                               v8 = iadd_imm v4, 4336
;; @0050                               v9 = imul_imm v6, 4
;; @0050                               v10 = imul_imm v6, 8
;; @0050                               v11 = iadd v7, v9
;; @0050                               v12 = iadd v8, v10
;; @0050                               v13 = load.i32 notrap aligned v11
;; @0050                               v14 = load.i64 notrap aligned v12
;; @0050                               v15 = icmp eq v13, v2
;; @0050                               brif v15, block3(v14, v4), block2
;;
;;                                 block2 cold:
;; @0050                               v16 = iconst.i32 10
;; @0050                               v17 = icmp.i32 uge v2, v16  ; v16 = 10
;; @0050                               v18 = uextend.i64 v2
;; @0050                               v19 = global_value.i64 gv4
;; @0050                               v20 = ishl_imm v18, 3
;; @0050                               v21 = iadd v19, v20
;; @0050                               v22 = iconst.i64 0
;; @0050                               v23 = select_spectre_guard v17, v22, v21  ; v22 = 0
;; @0050                               v24 = load.i64 table_oob aligned table v23
;; @0050                               v25 = band_imm v24, -2
;; @0050                               brif v24, block6(v25), block5
;;
;;                                 block4 cold:
;; @0050                               store.i32 notrap aligned v2, v11
;; @0050                               store.i64 notrap aligned v35, v12
;; @0050                               jump block3(v35, v36)
;;
;;                                 block3(v38: i64, v39: i64):
;; @0050                               v40 = call_indirect sig0, v38(v39, v0)
;; @0053                               jump block1(v40)
;;
;;                                 block5 cold:
;; @0050                               v27 = iconst.i32 0
;; @0050                               v28 = global_value.i64 gv3
;; @0050                               v29 = call fn0(v28, v27, v2)  ; v27 = 0
;; @0050                               jump block6(v29)
;;
;;                                 block6(v26: i64) cold:
;; @0050                               v30 = global_value.i64 gv3
;; @0050                               v31 = load.i64 notrap aligned readonly v30+80
;; @0050                               v32 = load.i32 notrap aligned readonly v31
;; @0050                               v33 = load.i32 icall_null aligned readonly v26+16
;; @0050                               v34 = icmp eq v33, v32
;; @0050                               trapz v34, bad_sig
;; @0050                               v35 = load.i64 notrap aligned readonly v26+8
;; @0050                               v36 = load.i64 notrap aligned readonly v26+24
;; @0050                               v37 = icmp eq v36, v4
;; @0050                               brif v37, block4, block3(v35, v36)
;;
;;                                 block1(v3: i32):
;; @0053                               return v3
;; }
