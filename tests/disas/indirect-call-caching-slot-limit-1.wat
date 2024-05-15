;;! target = "x86_64"
;;! flags = [ "-Ocache-call-indirects=y", "-Omax-call-indirect-cache-slots=2" ]

;; This test checks that we properly bound the number of call-indirect
;; cache slots. The first case (here) is when the limit falls in the
;; middle of a function. We set the limit to 2 above; we have 3
;; `call_indirect`s below; the last should not have caching code.
;;
;; In particular, below we see the cache probe sequence in block0
;; (first) and block3 (second); but the third call, starting in
;; block8, has no cache slot access and just performs the checks
;; unconditionally.

(module
 (table 10 10 funcref)

 (func (export "call_it") (param i32) (result i32)
  local.get 0
  call_indirect (result i32)
  call_indirect (result i32)
  call_indirect (result i32)))
  
;; function u0:0(i64 vmctx, i64, i32) -> i32 fast {
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
;; @0033                               v4 = global_value.i64 gv3
;; @0033                               v5 = iadd_imm v4, 144
;; @0033                               v6 = load.i32 notrap aligned v5+8
;; @0033                               v7 = load.i64 notrap aligned v5
;; @0033                               v8 = icmp eq v6, v2
;; @0033                               brif v8, block3(v7, v4), block2
;;
;;                                 block2 cold:
;; @0033                               v9 = iconst.i32 10
;; @0033                               v10 = icmp.i32 uge v2, v9  ; v9 = 10
;; @0033                               v11 = uextend.i64 v2
;; @0033                               v12 = global_value.i64 gv4
;; @0033                               v13 = ishl_imm v11, 3
;; @0033                               v14 = iadd v12, v13
;; @0033                               v15 = iconst.i64 0
;; @0033                               v16 = select_spectre_guard v10, v15, v14  ; v15 = 0
;; @0033                               v17 = load.i64 table_oob aligned table v16
;; @0033                               v18 = band_imm v17, -2
;; @0033                               brif v17, block6(v18), block5
;;
;;                                 block4 cold:
;; @0033                               store.i32 notrap aligned v2, v5+8
;; @0033                               store.i64 notrap aligned v28, v5
;; @0033                               jump block3(v28, v29)
;;
;;                                 block3(v31: i64, v32: i64):
;; @0033                               v33 = call_indirect sig0, v31(v32, v0)
;; @0036                               v34 = global_value.i64 gv3
;; @0036                               v35 = iadd_imm v34, 160
;; @0036                               v36 = load.i32 notrap aligned v35+8
;; @0036                               v37 = load.i64 notrap aligned v35
;; @0036                               v38 = icmp eq v36, v33
;; @0036                               brif v38, block8(v37, v34), block7
;;
;;                                 block7 cold:
;; @0036                               v39 = iconst.i32 10
;; @0036                               v40 = icmp.i32 uge v33, v39  ; v39 = 10
;; @0036                               v41 = uextend.i64 v33
;; @0036                               v42 = global_value.i64 gv4
;; @0036                               v43 = ishl_imm v41, 3
;; @0036                               v44 = iadd v42, v43
;; @0036                               v45 = iconst.i64 0
;; @0036                               v46 = select_spectre_guard v40, v45, v44  ; v45 = 0
;; @0036                               v47 = load.i64 table_oob aligned table v46
;; @0036                               v48 = band_imm v47, -2
;; @0036                               brif v47, block11(v48), block10
;;
;;                                 block9 cold:
;; @0036                               store.i32 notrap aligned v33, v35+8
;; @0036                               store.i64 notrap aligned v58, v35
;; @0036                               jump block8(v58, v59)
;;
;;                                 block8(v61: i64, v62: i64):
;; @0036                               v63 = call_indirect sig0, v61(v62, v0)
;; @0039                               v64 = iconst.i32 10
;; @0039                               v65 = icmp uge v63, v64  ; v64 = 10
;; @0039                               v66 = uextend.i64 v63
;; @0039                               v67 = global_value.i64 gv4
;; @0039                               v68 = ishl_imm v66, 3
;; @0039                               v69 = iadd v67, v68
;; @0039                               v70 = iconst.i64 0
;; @0039                               v71 = select_spectre_guard v65, v70, v69  ; v70 = 0
;; @0039                               v72 = load.i64 table_oob aligned table v71
;; @0039                               v73 = band_imm v72, -2
;; @0039                               brif v72, block13(v73), block12
;;
;;                                 block5 cold:
;; @0033                               v20 = iconst.i32 0
;; @0033                               v21 = global_value.i64 gv3
;; @0033                               v22 = call fn0(v21, v20, v2)  ; v20 = 0
;; @0033                               jump block6(v22)
;;
;;                                 block6(v19: i64) cold:
;; @0033                               v23 = global_value.i64 gv3
;; @0033                               v24 = load.i64 notrap aligned readonly v23+80
;; @0033                               v25 = load.i32 notrap aligned readonly v24+4
;; @0033                               v26 = load.i32 icall_null aligned readonly v19+16
;; @0033                               v27 = icmp eq v26, v25
;; @0033                               trapz v27, bad_sig
;; @0033                               v28 = load.i64 notrap aligned readonly v19+8
;; @0033                               v29 = load.i64 notrap aligned readonly v19+24
;; @0033                               v30 = icmp eq v29, v4
;; @0033                               brif v30, block4, block3(v28, v29)
;;
;;                                 block10 cold:
;; @0036                               v50 = iconst.i32 0
;; @0036                               v51 = global_value.i64 gv3
;; @0036                               v52 = call fn0(v51, v50, v33)  ; v50 = 0
;; @0036                               jump block11(v52)
;;
;;                                 block11(v49: i64) cold:
;; @0036                               v53 = global_value.i64 gv3
;; @0036                               v54 = load.i64 notrap aligned readonly v53+80
;; @0036                               v55 = load.i32 notrap aligned readonly v54+4
;; @0036                               v56 = load.i32 icall_null aligned readonly v49+16
;; @0036                               v57 = icmp eq v56, v55
;; @0036                               trapz v57, bad_sig
;; @0036                               v58 = load.i64 notrap aligned readonly v49+8
;; @0036                               v59 = load.i64 notrap aligned readonly v49+24
;; @0036                               v60 = icmp eq v59, v34
;; @0036                               brif v60, block9, block8(v58, v59)
;;
;;                                 block12 cold:
;; @0039                               v75 = iconst.i32 0
;; @0039                               v76 = global_value.i64 gv3
;; @0039                               v77 = call fn0(v76, v75, v63)  ; v75 = 0
;; @0039                               jump block13(v77)
;;
;;                                 block13(v74: i64):
;; @0039                               v78 = global_value.i64 gv3
;; @0039                               v79 = load.i64 notrap aligned readonly v78+80
;; @0039                               v80 = load.i32 notrap aligned readonly v79+4
;; @0039                               v81 = load.i32 icall_null aligned readonly v74+16
;; @0039                               v82 = icmp eq v81, v80
;; @0039                               trapz v82, bad_sig
;; @0039                               v83 = load.i64 notrap aligned readonly v74+8
;; @0039                               v84 = load.i64 notrap aligned readonly v74+24
;; @0039                               v85 = call_indirect sig0, v83(v84, v0)
;; @003c                               jump block1(v85)
;;
;;                                 block1(v3: i32):
;; @003c                               return v3
;; }
