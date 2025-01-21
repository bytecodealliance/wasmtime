;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-O opt-level=0"

;; Test basic code generation for table WebAssembly instructions.
;; Use optimization but with `opt-level=0` to legalize away table_addr instructions.

(module
  (table (export "table") 1 externref)
  (func (export "table.set.const") (param externref)
    i32.const 0
    local.get 0
    table.set 0)
  (func (export "table.set.var") (param i32 externref)
    local.get 0
    local.get 1
    table.set 0))

;; function u0:0(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i64 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0051                               v3 = iconst.i32 0
;; @0055                               v4 = load.i64 notrap aligned v0+96
;; @0055                               v5 = ireduce.i32 v4
;; @0055                               v6 = icmp uge v3, v5  ; v3 = 0
;; @0055                               v7 = uextend.i64 v3  ; v3 = 0
;; @0055                               v8 = load.i64 notrap aligned v0+88
;;                                     v68 = iconst.i64 2
;; @0055                               v9 = ishl v7, v68  ; v68 = 2
;; @0055                               v10 = iadd v8, v9
;; @0055                               v11 = iconst.i64 0
;; @0055                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @0055                               v13 = load.i32 user5 aligned table v12
;;                                     v69 = iconst.i32 0
;; @0055                               v14 = icmp eq v2, v69  ; v69 = 0
;; @0055                               brif v14, block3, block2
;;
;;                                 block2:
;; @0055                               v16 = load.i64 notrap aligned readonly v0+40
;; @0055                               v18 = load.i64 notrap aligned readonly v0+48
;; @0055                               v19 = uextend.i64 v2
;; @0055                               v20 = iconst.i64 8
;; @0055                               v21 = uadd_overflow_trap v19, v20, user1  ; v20 = 8
;; @0055                               v22 = iconst.i64 8
;; @0055                               v23 = uadd_overflow_trap v21, v22, user1  ; v22 = 8
;; @0055                               v24 = icmp ule v23, v18
;; @0055                               trapz v24, user1
;; @0055                               v25 = iadd v16, v21
;; @0055                               v26 = load.i64 notrap aligned v25
;;                                     v70 = iconst.i64 1
;; @0055                               v27 = iadd v26, v70  ; v70 = 1
;; @0055                               v29 = load.i64 notrap aligned readonly v0+40
;; @0055                               v31 = load.i64 notrap aligned readonly v0+48
;; @0055                               v32 = uextend.i64 v2
;; @0055                               v33 = iconst.i64 8
;; @0055                               v34 = uadd_overflow_trap v32, v33, user1  ; v33 = 8
;; @0055                               v35 = iconst.i64 8
;; @0055                               v36 = uadd_overflow_trap v34, v35, user1  ; v35 = 8
;; @0055                               v37 = icmp ule v36, v31
;; @0055                               trapz v37, user1
;; @0055                               v38 = iadd v29, v34
;; @0055                               store notrap aligned v27, v38
;; @0055                               jump block3
;;
;;                                 block3:
;; @0055                               store.i32 user5 aligned table v2, v12
;;                                     v71 = iconst.i32 0
;; @0055                               v39 = icmp.i32 eq v13, v71  ; v71 = 0
;; @0055                               brif v39, block7, block4
;;
;;                                 block4:
;; @0055                               v41 = load.i64 notrap aligned readonly v0+40
;; @0055                               v43 = load.i64 notrap aligned readonly v0+48
;; @0055                               v44 = uextend.i64 v13
;; @0055                               v45 = iconst.i64 8
;; @0055                               v46 = uadd_overflow_trap v44, v45, user1  ; v45 = 8
;; @0055                               v47 = iconst.i64 8
;; @0055                               v48 = uadd_overflow_trap v46, v47, user1  ; v47 = 8
;; @0055                               v49 = icmp ule v48, v43
;; @0055                               trapz v49, user1
;; @0055                               v50 = iadd v41, v46
;; @0055                               v51 = load.i64 notrap aligned v50
;;                                     v72 = iconst.i64 -1
;; @0055                               v52 = iadd v51, v72  ; v72 = -1
;;                                     v73 = iconst.i64 0
;; @0055                               v53 = icmp eq v52, v73  ; v73 = 0
;; @0055                               brif v53, block5, block6
;;
;;                                 block5 cold:
;; @0055                               call fn0(v0, v13)
;; @0055                               jump block7
;;
;;                                 block6:
;; @0055                               v56 = load.i64 notrap aligned readonly v0+40
;; @0055                               v58 = load.i64 notrap aligned readonly v0+48
;; @0055                               v59 = uextend.i64 v13
;; @0055                               v60 = iconst.i64 8
;; @0055                               v61 = uadd_overflow_trap v59, v60, user1  ; v60 = 8
;; @0055                               v62 = iconst.i64 8
;; @0055                               v63 = uadd_overflow_trap v61, v62, user1  ; v62 = 8
;; @0055                               v64 = icmp ule v63, v58
;; @0055                               trapz v64, user1
;; @0055                               v65 = iadd v56, v61
;; @0055                               store.i64 notrap aligned v52, v65
;; @0055                               jump block7
;;
;;                                 block7:
;; @0057                               jump block1
;;
;;                                 block1:
;; @0057                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i64 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @005e                               v4 = load.i64 notrap aligned v0+96
;; @005e                               v5 = ireduce.i32 v4
;; @005e                               v6 = icmp uge v2, v5
;; @005e                               v7 = uextend.i64 v2
;; @005e                               v8 = load.i64 notrap aligned v0+88
;;                                     v68 = iconst.i64 2
;; @005e                               v9 = ishl v7, v68  ; v68 = 2
;; @005e                               v10 = iadd v8, v9
;; @005e                               v11 = iconst.i64 0
;; @005e                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @005e                               v13 = load.i32 user5 aligned table v12
;;                                     v69 = iconst.i32 0
;; @005e                               v14 = icmp eq v3, v69  ; v69 = 0
;; @005e                               brif v14, block3, block2
;;
;;                                 block2:
;; @005e                               v16 = load.i64 notrap aligned readonly v0+40
;; @005e                               v18 = load.i64 notrap aligned readonly v0+48
;; @005e                               v19 = uextend.i64 v3
;; @005e                               v20 = iconst.i64 8
;; @005e                               v21 = uadd_overflow_trap v19, v20, user1  ; v20 = 8
;; @005e                               v22 = iconst.i64 8
;; @005e                               v23 = uadd_overflow_trap v21, v22, user1  ; v22 = 8
;; @005e                               v24 = icmp ule v23, v18
;; @005e                               trapz v24, user1
;; @005e                               v25 = iadd v16, v21
;; @005e                               v26 = load.i64 notrap aligned v25
;;                                     v70 = iconst.i64 1
;; @005e                               v27 = iadd v26, v70  ; v70 = 1
;; @005e                               v29 = load.i64 notrap aligned readonly v0+40
;; @005e                               v31 = load.i64 notrap aligned readonly v0+48
;; @005e                               v32 = uextend.i64 v3
;; @005e                               v33 = iconst.i64 8
;; @005e                               v34 = uadd_overflow_trap v32, v33, user1  ; v33 = 8
;; @005e                               v35 = iconst.i64 8
;; @005e                               v36 = uadd_overflow_trap v34, v35, user1  ; v35 = 8
;; @005e                               v37 = icmp ule v36, v31
;; @005e                               trapz v37, user1
;; @005e                               v38 = iadd v29, v34
;; @005e                               store notrap aligned v27, v38
;; @005e                               jump block3
;;
;;                                 block3:
;; @005e                               store.i32 user5 aligned table v3, v12
;;                                     v71 = iconst.i32 0
;; @005e                               v39 = icmp.i32 eq v13, v71  ; v71 = 0
;; @005e                               brif v39, block7, block4
;;
;;                                 block4:
;; @005e                               v41 = load.i64 notrap aligned readonly v0+40
;; @005e                               v43 = load.i64 notrap aligned readonly v0+48
;; @005e                               v44 = uextend.i64 v13
;; @005e                               v45 = iconst.i64 8
;; @005e                               v46 = uadd_overflow_trap v44, v45, user1  ; v45 = 8
;; @005e                               v47 = iconst.i64 8
;; @005e                               v48 = uadd_overflow_trap v46, v47, user1  ; v47 = 8
;; @005e                               v49 = icmp ule v48, v43
;; @005e                               trapz v49, user1
;; @005e                               v50 = iadd v41, v46
;; @005e                               v51 = load.i64 notrap aligned v50
;;                                     v72 = iconst.i64 -1
;; @005e                               v52 = iadd v51, v72  ; v72 = -1
;;                                     v73 = iconst.i64 0
;; @005e                               v53 = icmp eq v52, v73  ; v73 = 0
;; @005e                               brif v53, block5, block6
;;
;;                                 block5 cold:
;; @005e                               call fn0(v0, v13)
;; @005e                               jump block7
;;
;;                                 block6:
;; @005e                               v56 = load.i64 notrap aligned readonly v0+40
;; @005e                               v58 = load.i64 notrap aligned readonly v0+48
;; @005e                               v59 = uextend.i64 v13
;; @005e                               v60 = iconst.i64 8
;; @005e                               v61 = uadd_overflow_trap v59, v60, user1  ; v60 = 8
;; @005e                               v62 = iconst.i64 8
;; @005e                               v63 = uadd_overflow_trap v61, v62, user1  ; v62 = 8
;; @005e                               v64 = icmp ule v63, v58
;; @005e                               trapz v64, user1
;; @005e                               v65 = iadd v56, v61
;; @005e                               store.i64 notrap aligned v52, v65
;; @005e                               jump block7
;;
;;                                 block7:
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
