;;! target = "x86_64"
;;! flags = [ "-Ocache-call-indirects=y", "-Omax-call-indirect-cache-slots=2" ]

;; This test checks that we properly bound the number of call-indirect
;; cache slots. The second case (here) is when the limit falls
;; entirely before a function. We set the limit to 2 above; we have 2
;; callsites in the first function; the second function should have no
;; caching.
;;
;; In particular, below we see the cache probe sequence in block0
;; (first) and block3 (second) in `u0:0` (`call_it`); but the call in
;; the second function in `u0:1` (`call_it_2`), starting in block0 in
;; that function, has no cache slot access and just performs the
;; checks unconditionally.

(module
 (table 10 10 funcref)

 (func (export "call_it") (param i32) (result i32)
  local.get 0
  call_indirect (result i32)
  call_indirect (result i32))
 
 (func (export "call_it_2") (param i32) (result i32)
  local.get 0
  call_indirect (result i32)))
  
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
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
;; @0040                               v4 = global_value.i64 gv3
;; @0040                               v5 = iadd_imm v4, 176
;; @0040                               v6 = load.i32 notrap aligned v5+8
;; @0040                               v7 = load.i64 notrap aligned v5
;; @0040                               v8 = icmp eq v6, v2
;; @0040                               brif v8, block3(v7, v4), block2
;;
;;                                 block2 cold:
;; @0040                               v9 = iconst.i32 10
;; @0040                               v10 = icmp.i32 uge v2, v9  ; v9 = 10
;; @0040                               v11 = uextend.i64 v2
;; @0040                               v12 = global_value.i64 gv4
;; @0040                               v13 = ishl_imm v11, 3
;; @0040                               v14 = iadd v12, v13
;; @0040                               v15 = iconst.i64 0
;; @0040                               v16 = select_spectre_guard v10, v15, v14  ; v15 = 0
;; @0040                               v17 = load.i64 table_oob aligned table v16
;; @0040                               v18 = band_imm v17, -2
;; @0040                               brif v17, block6(v18), block5
;;
;;                                 block4 cold:
;; @0040                               store.i32 notrap aligned v2, v5+8
;; @0040                               store.i64 notrap aligned v28, v5
;; @0040                               jump block3(v28, v29)
;;
;;                                 block3(v31: i64, v32: i64):
;; @0040                               v33 = call_indirect sig0, v31(v32, v0)
;; @0043                               v34 = global_value.i64 gv3
;; @0043                               v35 = iadd_imm v34, 192
;; @0043                               v36 = load.i32 notrap aligned v35+8
;; @0043                               v37 = load.i64 notrap aligned v35
;; @0043                               v38 = icmp eq v36, v33
;; @0043                               brif v38, block8(v37, v34), block7
;;
;;                                 block7 cold:
;; @0043                               v39 = iconst.i32 10
;; @0043                               v40 = icmp.i32 uge v33, v39  ; v39 = 10
;; @0043                               v41 = uextend.i64 v33
;; @0043                               v42 = global_value.i64 gv4
;; @0043                               v43 = ishl_imm v41, 3
;; @0043                               v44 = iadd v42, v43
;; @0043                               v45 = iconst.i64 0
;; @0043                               v46 = select_spectre_guard v40, v45, v44  ; v45 = 0
;; @0043                               v47 = load.i64 table_oob aligned table v46
;; @0043                               v48 = band_imm v47, -2
;; @0043                               brif v47, block11(v48), block10
;;
;;                                 block9 cold:
;; @0043                               store.i32 notrap aligned v33, v35+8
;; @0043                               store.i64 notrap aligned v58, v35
;; @0043                               jump block8(v58, v59)
;;
;;                                 block8(v61: i64, v62: i64):
;; @0043                               v63 = call_indirect sig0, v61(v62, v0)
;; @0046                               jump block1(v63)
;;
;;                                 block5 cold:
;; @0040                               v20 = iconst.i32 0
;; @0040                               v21 = global_value.i64 gv3
;; @0040                               v22 = call fn0(v21, v20, v2)  ; v20 = 0
;; @0040                               jump block6(v22)
;;
;;                                 block6(v19: i64) cold:
;; @0040                               v23 = global_value.i64 gv3
;; @0040                               v24 = load.i64 notrap aligned readonly v23+80
;; @0040                               v25 = load.i32 notrap aligned readonly v24+4
;; @0040                               v26 = load.i32 icall_null aligned readonly v19+16
;; @0040                               v27 = icmp eq v26, v25
;; @0040                               trapz v27, bad_sig
;; @0040                               v28 = load.i64 notrap aligned readonly v19+8
;; @0040                               v29 = load.i64 notrap aligned readonly v19+24
;; @0040                               v30 = icmp eq v29, v4
;; @0040                               brif v30, block4, block3(v28, v29)
;;
;;                                 block10 cold:
;; @0043                               v50 = iconst.i32 0
;; @0043                               v51 = global_value.i64 gv3
;; @0043                               v52 = call fn0(v51, v50, v33)  ; v50 = 0
;; @0043                               jump block11(v52)
;;
;;                                 block11(v49: i64) cold:
;; @0043                               v53 = global_value.i64 gv3
;; @0043                               v54 = load.i64 notrap aligned readonly v53+80
;; @0043                               v55 = load.i32 notrap aligned readonly v54+4
;; @0043                               v56 = load.i32 icall_null aligned readonly v49+16
;; @0043                               v57 = icmp eq v56, v55
;; @0043                               trapz v57, bad_sig
;; @0043                               v58 = load.i64 notrap aligned readonly v49+8
;; @0043                               v59 = load.i64 notrap aligned readonly v49+24
;; @0043                               v60 = icmp eq v59, v34
;; @0043                               brif v60, block9, block8(v58, v59)
;;
;;                                 block1(v3: i32):
;; @0046                               return v3
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
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
;; @004b                               v4 = iconst.i32 10
;; @004b                               v5 = icmp uge v2, v4  ; v4 = 10
;; @004b                               v6 = uextend.i64 v2
;; @004b                               v7 = global_value.i64 gv4
;; @004b                               v8 = ishl_imm v6, 3
;; @004b                               v9 = iadd v7, v8
;; @004b                               v10 = iconst.i64 0
;; @004b                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @004b                               v12 = load.i64 table_oob aligned table v11
;; @004b                               v13 = band_imm v12, -2
;; @004b                               brif v12, block3(v13), block2
;;
;;                                 block2 cold:
;; @004b                               v15 = iconst.i32 0
;; @004b                               v16 = global_value.i64 gv3
;; @004b                               v17 = call fn0(v16, v15, v2)  ; v15 = 0
;; @004b                               jump block3(v17)
;;
;;                                 block3(v14: i64):
;; @004b                               v18 = global_value.i64 gv3
;; @004b                               v19 = load.i64 notrap aligned readonly v18+80
;; @004b                               v20 = load.i32 notrap aligned readonly v19+4
;; @004b                               v21 = load.i32 icall_null aligned readonly v14+16
;; @004b                               v22 = icmp eq v21, v20
;; @004b                               trapz v22, bad_sig
;; @004b                               v23 = load.i64 notrap aligned readonly v14+8
;; @004b                               v24 = load.i64 notrap aligned readonly v14+24
;; @004b                               v25 = call_indirect sig0, v23(v24, v0)
;; @004e                               jump block1(v25)
;;
;;                                 block1(v3: i32):
;; @004e                               return v3
;; }
