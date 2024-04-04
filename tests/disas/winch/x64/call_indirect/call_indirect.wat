;;! target="x86_64"

(module
  (type $over-i32 (func (param i32) (result i32)))

  (table funcref
    (elem
      $fib-i32
    )
  )
  
  (func $fib-i32 (export "fib-i32") (type $over-i32)
    (if (result i32) (i32.le_u (local.get 0) (i32.const 1))
      (then (i32.const 1))
      (else
        (i32.add
          (call_indirect (type $over-i32)
            (i32.sub (local.get 0) (i32.const 2))
            (i32.const 0)
          )
          (call_indirect (type $over-i32)
            (i32.sub (local.get 0) (i32.const 1))
            (i32.const 0)
          )
        )
      )
    )
  )
)


;; function u0:0(i64 vmctx, i64, i32) -> i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i64, i32) -> i32 fast
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     fn0 = colocated u1:9 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0038                               v4 = iconst.i32 1
;; @003a                               v5 = icmp ule v2, v4  ; v4 = 1
;; @003a                               v6 = uextend.i32 v5
;; @003b                               brif v6, block2, block4
;;
;;                                 block2:
;; @003d                               v8 = iconst.i32 1
;; @003f                               jump block3(v8)  ; v8 = 1
;;
;;                                 block4:
;; @0042                               v9 = iconst.i32 2
;; @0044                               v10 = isub.i32 v2, v9  ; v9 = 2
;; @0045                               v11 = iconst.i32 0
;; @0047                               v12 = iconst.i32 1
;; @0047                               v13 = icmp uge v11, v12  ; v11 = 0, v12 = 1
;; @0047                               v14 = uextend.i64 v11  ; v11 = 0
;; @0047                               v15 = global_value.i64 gv4
;; @0047                               v16 = ishl_imm v14, 3
;; @0047                               v17 = iadd v15, v16
;; @0047                               v18 = iconst.i64 0
;; @0047                               v19 = select_spectre_guard v13, v18, v17  ; v18 = 0
;; @0047                               v20 = load.i64 table_oob aligned table v19
;; @0047                               v21 = band_imm v20, -2
;; @0047                               brif v20, block6(v21), block5
;;
;;                                 block5 cold:
;; @0047                               v23 = iconst.i32 0
;; @0047                               v24 = global_value.i64 gv3
;; @0047                               v25 = call fn0(v24, v23, v11)  ; v23 = 0, v11 = 0
;; @0047                               jump block6(v25)
;;
;;                                 block6(v22: i64):
;; @0047                               v26 = global_value.i64 gv3
;; @0047                               v27 = load.i64 notrap aligned readonly v26+80
;; @0047                               v28 = load.i32 notrap aligned readonly v27
;; @0047                               v29 = load.i32 icall_null aligned readonly v22+24
;; @0047                               v30 = icmp eq v29, v28
;; @0047                               trapz v30, bad_sig
;; @0047                               v31 = load.i64 notrap aligned readonly v22+16
;; @0047                               v32 = load.i64 notrap aligned readonly v22+32
;; @0047                               v33 = call_indirect sig0, v31(v32, v0, v10)
;; @004c                               v35 = iconst.i32 1
;; @004e                               v36 = isub.i32 v2, v35  ; v35 = 1
;; @004f                               v37 = iconst.i32 0
;; @0051                               v38 = iconst.i32 1
;; @0051                               v39 = icmp uge v37, v38  ; v37 = 0, v38 = 1
;; @0051                               v40 = uextend.i64 v37  ; v37 = 0
;; @0051                               v41 = global_value.i64 gv4
;; @0051                               v42 = ishl_imm v40, 3
;; @0051                               v43 = iadd v41, v42
;; @0051                               v44 = iconst.i64 0
;; @0051                               v45 = select_spectre_guard v39, v44, v43  ; v44 = 0
;; @0051                               v46 = load.i64 table_oob aligned table v45
;; @0051                               v47 = band_imm v46, -2
;; @0051                               brif v46, block8(v47), block7
;;
;;                                 block7 cold:
;; @0051                               v49 = iconst.i32 0
;; @0051                               v50 = global_value.i64 gv3
;; @0051                               v51 = call fn0(v50, v49, v37)  ; v49 = 0, v37 = 0
;; @0051                               jump block8(v51)
;;
;;                                 block8(v48: i64):
;; @0051                               v52 = global_value.i64 gv3
;; @0051                               v53 = load.i64 notrap aligned readonly v52+80
;; @0051                               v54 = load.i32 notrap aligned readonly v53
;; @0051                               v55 = load.i32 icall_null aligned readonly v48+24
;; @0051                               v56 = icmp eq v55, v54
;; @0051                               trapz v56, bad_sig
;; @0051                               v57 = load.i64 notrap aligned readonly v48+16
;; @0051                               v58 = load.i64 notrap aligned readonly v48+32
;; @0051                               v59 = call_indirect sig0, v57(v58, v0, v36)
;; @0054                               v60 = iadd.i32 v33, v59
;; @0055                               jump block3(v60)
;;
;;                                 block3(v7: i32):
;; @0056                               jump block1(v7)
;;
;;                                 block1(v3: i32):
;; @0056                               return v3
;; }
