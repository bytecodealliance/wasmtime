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
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i64 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, i32 uext) -> i64 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0051                               v3 = iconst.i32 0
;; @0053                               v4 = load.i64 notrap aligned v0+96
;; @0053                               v5 = ireduce.i32 v4
;; @0053                               v6 = icmp uge v3, v5  ; v3 = 0
;; @0053                               v7 = uextend.i64 v3  ; v3 = 0
;; @0053                               v8 = load.i64 notrap aligned v0+88
;;                                     v55 = iconst.i64 2
;; @0053                               v9 = ishl v7, v55  ; v55 = 2
;; @0053                               v10 = iadd v8, v9
;; @0053                               v11 = iconst.i64 0
;; @0053                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @0053                               v13 = load.i32 user5 aligned table v12
;;                                     v56 = stack_addr.i64 ss0
;;                                     store notrap v13, v56
;;                                     v57 = stack_addr.i64 ss0
;;                                     v52 = load.i32 notrap v57
;;                                     v58 = iconst.i32 0
;; @0053                               v14 = icmp eq v52, v58  ; v58 = 0
;; @0053                               brif v14, block5, block2
;;
;;                                 block2:
;; @0053                               v16 = load.i64 notrap aligned readonly v0+56
;; @0053                               v17 = load.i64 notrap aligned v16
;; @0053                               v18 = load.i64 notrap aligned v16+8
;; @0053                               v19 = icmp eq v17, v18
;; @0053                               brif v19, block3, block4
;;
;;                                 block4:
;; @0053                               v21 = load.i64 notrap aligned readonly v0+40
;; @0053                               v23 = load.i64 notrap aligned readonly v0+48
;;                                     v59 = stack_addr.i64 ss0
;;                                     v51 = load.i32 notrap v59
;; @0053                               v24 = uextend.i64 v51
;; @0053                               v25 = iconst.i64 8
;; @0053                               v26 = uadd_overflow_trap v24, v25, user1  ; v25 = 8
;; @0053                               v27 = iconst.i64 8
;; @0053                               v28 = uadd_overflow_trap v26, v27, user1  ; v27 = 8
;; @0053                               v29 = icmp ule v28, v23
;; @0053                               trapz v29, user1
;; @0053                               v30 = iadd v21, v26
;; @0053                               v31 = load.i64 notrap aligned v30
;;                                     v60 = iconst.i64 1
;; @0053                               v32 = iadd v31, v60  ; v60 = 1
;; @0053                               v34 = load.i64 notrap aligned readonly v0+40
;; @0053                               v36 = load.i64 notrap aligned readonly v0+48
;;                                     v61 = stack_addr.i64 ss0
;;                                     v50 = load.i32 notrap v61
;; @0053                               v37 = uextend.i64 v50
;; @0053                               v38 = iconst.i64 8
;; @0053                               v39 = uadd_overflow_trap v37, v38, user1  ; v38 = 8
;; @0053                               v40 = iconst.i64 8
;; @0053                               v41 = uadd_overflow_trap v39, v40, user1  ; v40 = 8
;; @0053                               v42 = icmp ule v41, v36
;; @0053                               trapz v42, user1
;; @0053                               v43 = iadd v34, v39
;; @0053                               store notrap aligned v32, v43
;;                                     v62 = stack_addr.i64 ss0
;;                                     v49 = load.i32 notrap v62
;; @0053                               store notrap aligned v49, v17
;;                                     v63 = iconst.i64 4
;; @0053                               v44 = iadd.i64 v17, v63  ; v63 = 4
;; @0053                               store notrap aligned v44, v16
;; @0053                               jump block5
;;
;;                                 block3 cold:
;;                                     v64 = stack_addr.i64 ss0
;;                                     v48 = load.i32 notrap v64
;; @0053                               v46 = call fn0(v0, v48), stack_map=[i32 @ ss0+0]
;; @0053                               jump block5
;;
;;                                 block5:
;;                                     v65 = stack_addr.i64 ss0
;;                                     v47 = load.i32 notrap v65
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return v47
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i64 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, i32 uext) -> i64 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005a                               v4 = load.i64 notrap aligned v0+96
;; @005a                               v5 = ireduce.i32 v4
;; @005a                               v6 = icmp uge v2, v5
;; @005a                               v7 = uextend.i64 v2
;; @005a                               v8 = load.i64 notrap aligned v0+88
;;                                     v55 = iconst.i64 2
;; @005a                               v9 = ishl v7, v55  ; v55 = 2
;; @005a                               v10 = iadd v8, v9
;; @005a                               v11 = iconst.i64 0
;; @005a                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @005a                               v13 = load.i32 user5 aligned table v12
;;                                     v56 = stack_addr.i64 ss0
;;                                     store notrap v13, v56
;;                                     v57 = stack_addr.i64 ss0
;;                                     v52 = load.i32 notrap v57
;;                                     v58 = iconst.i32 0
;; @005a                               v14 = icmp eq v52, v58  ; v58 = 0
;; @005a                               brif v14, block5, block2
;;
;;                                 block2:
;; @005a                               v16 = load.i64 notrap aligned readonly v0+56
;; @005a                               v17 = load.i64 notrap aligned v16
;; @005a                               v18 = load.i64 notrap aligned v16+8
;; @005a                               v19 = icmp eq v17, v18
;; @005a                               brif v19, block3, block4
;;
;;                                 block4:
;; @005a                               v21 = load.i64 notrap aligned readonly v0+40
;; @005a                               v23 = load.i64 notrap aligned readonly v0+48
;;                                     v59 = stack_addr.i64 ss0
;;                                     v51 = load.i32 notrap v59
;; @005a                               v24 = uextend.i64 v51
;; @005a                               v25 = iconst.i64 8
;; @005a                               v26 = uadd_overflow_trap v24, v25, user1  ; v25 = 8
;; @005a                               v27 = iconst.i64 8
;; @005a                               v28 = uadd_overflow_trap v26, v27, user1  ; v27 = 8
;; @005a                               v29 = icmp ule v28, v23
;; @005a                               trapz v29, user1
;; @005a                               v30 = iadd v21, v26
;; @005a                               v31 = load.i64 notrap aligned v30
;;                                     v60 = iconst.i64 1
;; @005a                               v32 = iadd v31, v60  ; v60 = 1
;; @005a                               v34 = load.i64 notrap aligned readonly v0+40
;; @005a                               v36 = load.i64 notrap aligned readonly v0+48
;;                                     v61 = stack_addr.i64 ss0
;;                                     v50 = load.i32 notrap v61
;; @005a                               v37 = uextend.i64 v50
;; @005a                               v38 = iconst.i64 8
;; @005a                               v39 = uadd_overflow_trap v37, v38, user1  ; v38 = 8
;; @005a                               v40 = iconst.i64 8
;; @005a                               v41 = uadd_overflow_trap v39, v40, user1  ; v40 = 8
;; @005a                               v42 = icmp ule v41, v36
;; @005a                               trapz v42, user1
;; @005a                               v43 = iadd v34, v39
;; @005a                               store notrap aligned v32, v43
;;                                     v62 = stack_addr.i64 ss0
;;                                     v49 = load.i32 notrap v62
;; @005a                               store notrap aligned v49, v17
;;                                     v63 = iconst.i64 4
;; @005a                               v44 = iadd.i64 v17, v63  ; v63 = 4
;; @005a                               store notrap aligned v44, v16
;; @005a                               jump block5
;;
;;                                 block3 cold:
;;                                     v64 = stack_addr.i64 ss0
;;                                     v48 = load.i32 notrap v64
;; @005a                               v46 = call fn0(v0, v48), stack_map=[i32 @ ss0+0]
;; @005a                               jump block5
;;
;;                                 block5:
;;                                     v65 = stack_addr.i64 ss0
;;                                     v47 = load.i32 notrap v65
;; @005c                               jump block1
;;
;;                                 block1:
;; @005c                               return v47
;; }
