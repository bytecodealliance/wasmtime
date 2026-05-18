;;! target = "x86_64"
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (data $passive "this is a passive data segment")
  (type $a (array i8))

  (func $a (param i32 i32) (result (ref $a))
    local.get 0
    local.get 1
    array.new_data $a $passive)
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig2 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:26 sig1
;;     fn2 = colocated u805306368:4 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0025                               v5 = uextend.i64 v3
;;                                     v72 = iconst.i64 32
;; @0025                               v7 = ushr v5, v72  ; v72 = 32
;; @0025                               trapnz v7, heap_oob
;; @0025                               v10 = uload32 notrap aligned v0+48
;; @0025                               v11 = uextend.i64 v2
;; @0025                               v13 = iadd v11, v5
;; @0025                               v14 = icmp ugt v13, v10
;; @0025                               trapnz v14, heap_oob
;; @0025                               v15 = iconst.i32 0
;; @0025                               v16 = call fn0(v0, v15, v2, v3)  ; v15 = 0
;; @0025                               trapnz v7, user18
;; @0025                               v17 = iconst.i32 28
;; @0025                               v22 = uadd_overflow_trap v17, v3, user18  ; v17 = 28
;; @0025                               v24 = iconst.i32 -1476395008
;; @0025                               v26 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v27 = load.i32 notrap aligned readonly can_move v26
;; @0025                               v28 = iconst.i32 8
;; @0025                               v29 = call fn1(v0, v24, v27, v22, v28)  ; v24 = -1476395008, v28 = 8
;;                                     v69 = stack_addr.i64 ss0
;;                                     store notrap v29, v69
;; @0025                               v67 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v30 = load.i64 notrap aligned readonly can_move v67+32
;; @0025                               v31 = uextend.i64 v29
;; @0025                               v32 = iadd v30, v31
;;                                     v65 = iconst.i64 24
;; @0025                               v33 = iadd v32, v65  ; v65 = 24
;; @0025                               store user2 v3, v33
;; @0025                               v41 = ushr v5, v72  ; v72 = 32
;; @0025                               trapnz v41, user2
;; @0025                               v44 = uadd_overflow_trap v3, v17, user2  ; v17 = 28
;;                                     v58 = load.i32 notrap v69
;; @0025                               v48 = uadd_overflow_trap v58, v44, user2
;; @0025                               v49 = uextend.i64 v48
;; @0025                               v51 = iadd v30, v49
;; @0025                               v52 = isub v44, v17  ; v17 = 28
;; @0025                               v53 = uextend.i64 v52
;; @0025                               v54 = isub v51, v53
;; @0025                               call fn2(v0, v54, v16, v5), stack_map=[i32 @ ss0+0]
;;                                     v57 = load.i32 notrap v69
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v57
;; }

;; function u0:0(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:24 sig0
;;     fn1 = colocated u805306368:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0025                               v6 = uload32 notrap aligned v0+56
;; @0025                               v7 = uextend.i64 v2
;; @0025                               v8 = uextend.i64 v3
;; @0025                               v10 = iadd v7, v8
;; @0025                               v11 = icmp ugt v10, v6
;; @0025                               trapnz v11, heap_oob
;; @0025                               v13 = load.i64 notrap aligned v0+48
;;                                     v101 = iconst.i64 32
;; @0025                               v19 = ushr v8, v101  ; v101 = 32
;; @0025                               trapnz v19, user18
;; @0025                               v16 = iconst.i32 28
;; @0025                               v21 = uadd_overflow_trap v16, v3, user18  ; v16 = 28
;; @0025                               v23 = iconst.i32 -1476395008
;; @0025                               v25 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v26 = load.i32 notrap aligned readonly can_move v25
;; @0025                               v27 = iconst.i32 8
;; @0025                               v28 = call fn0(v0, v23, v26, v21, v27)  ; v23 = -1476395008, v27 = 8
;;                                     v100 = stack_addr.i64 ss0
;;                                     store notrap v28, v100
;; @0025                               v98 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v29 = load.i64 notrap aligned readonly can_move v98+32
;; @0025                               v30 = uextend.i64 v28
;; @0025                               v31 = iadd v29, v30
;;                                     v96 = iconst.i64 24
;; @0025                               v32 = iadd v31, v96  ; v96 = 24
;; @0025                               store user2 v3, v32
;;                                     v76 = load.i32 notrap v100
;; @0025                               trapz v76, user16
;; @0025                               v34 = uextend.i64 v76
;; @0025                               v36 = iadd v29, v34
;; @0025                               v38 = iadd v36, v96  ; v96 = 24
;; @0025                               v39 = load.i32 user2 readonly v38
;; @0025                               v40 = uextend.i64 v39
;; @0025                               v45 = icmp ugt v8, v40
;; @0025                               trapnz v45, user17
;; @0025                               v54 = uload32 notrap aligned v0+56
;; @0025                               v59 = icmp ugt v10, v54
;; @0025                               trapnz v59, heap_oob
;; @0025                               v61 = load.i64 notrap aligned v0+48
;; @0025                               v68 = load.i64 notrap aligned v98+40
;;                                     v87 = iconst.i64 28
;; @0025                               v49 = iadd v36, v87  ; v87 = 28
;; @0025                               v70 = uadd_overflow_trap v49, v8, user2
;; @0025                               v69 = iadd v29, v68
;; @0025                               v71 = icmp ugt v70, v69
;; @0025                               trapnz v71, user2
;; @0025                               v63 = iadd v61, v7
;; @0025                               call fn1(v0, v49, v63, v8), stack_map=[i32 @ ss0+0]
;;                                     v73 = load.i32 notrap v100
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v73
;; }
