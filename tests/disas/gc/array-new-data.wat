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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 56 "VMContext+0x38"
;;     region3 = 48 "VMContext+0x30"
;;     region4 = 32 "VMContext+0x20"
;;     region5 = 40 "VMContext+0x28"
;;     region6 = 268435488 "VMStoreContext+0x20"
;;     region7 = 2147483648 "GcHeap"
;;     region8 = 268435496 "VMStoreContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:24 sig0
;;     fn1 = colocated u805306368:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0025                               v5 = load.i32 notrap aligned region2 v0+56
;; @0025                               v7 = uextend.i64 v2
;; @0025                               v8 = uextend.i64 v3
;; @0025                               v11 = iadd v7, v8
;; @0025                               v6 = uextend.i64 v5
;; @0025                               v12 = icmp ugt v11, v6
;; @0025                               trapnz v12, heap_oob
;; @0025                               v13 = load.i64 notrap aligned region3 v0+48
;; @0025                               v20 = iconst.i64 32
;; @0025                               v21 = ushr v8, v20  ; v20 = 32
;; @0025                               trapnz v21, user18
;; @0025                               v16 = iconst.i32 20
;; @0025                               v23 = uadd_overflow_trap v16, v3, user18  ; v16 = 20
;; @0025                               v24 = load.i64 notrap aligned readonly can_move region4 v0+32
;; @0025                               v25 = load.i32 notrap aligned v24
;; @0025                               v26 = load.i32 notrap aligned v24+4
;; @0025                               v32 = uextend.i64 v25
;; @0025                               v27 = uextend.i64 v23
;; @0025                               v28 = iconst.i64 15
;; @0025                               v30 = iadd v27, v28  ; v28 = 15
;; @0025                               v29 = iconst.i64 -16
;; @0025                               v31 = band v30, v29  ; v29 = -16
;; @0025                               v33 = iadd v32, v31
;; @0025                               v34 = uextend.i64 v26
;; @0025                               v35 = icmp ule v33, v34
;; @0025                               brif v35, block2, block3
;;
;;                                 block2:
;;                                     v121 = iconst.i32 15
;;                                     v122 = iadd.i32 v23, v121  ; v121 = 15
;;                                     v125 = iconst.i32 -16
;;                                     v126 = band v122, v125  ; v125 = -16
;;                                     v128 = iadd.i32 v25, v126
;; @0025                               store notrap aligned v128, v24
;;                                     v142 = iconst.i32 -1476395002
;;                                     v143 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v144 = load.i64 notrap aligned readonly can_move region6 v143+32
;; @0025                               v49 = iadd v144, v32
;; @0025                               store notrap aligned v142, v49  ; v142 = -1476395002
;;                                     v145 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v146 = load.i32 notrap aligned readonly can_move v145
;; @0025                               store notrap aligned v146, v49+4
;;                                     v147 = band.i64 v30, v29  ; v29 = -16
;; @0025                               istore32 notrap aligned v147, v49+8
;; @0025                               jump block4(v25, v49)
;;
;;                                 block3 cold:
;; @0025                               v36 = iconst.i32 -1476395002
;; @0025                               v37 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @0025                               v38 = load.i32 notrap aligned readonly can_move v37
;; @0025                               v39 = iconst.i32 16
;; @0025                               v40 = call fn0(v0, v36, v38, v23, v39)  ; v36 = -1476395002, v39 = 16
;; @0025                               v41 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v42 = load.i64 notrap aligned readonly can_move region6 v41+32
;; @0025                               v43 = uextend.i64 v40
;; @0025                               v44 = iadd v42, v43
;; @0025                               jump block4(v40, v44)
;;
;;                                 block4(v53: i32, v54: i64):
;;                                     v113 = stack_addr.i64 ss0
;;                                     store notrap v53, v113
;; @0025                               v55 = iconst.i64 16
;; @0025                               v56 = iadd v54, v55  ; v55 = 16
;; @0025                               store.i32 user2 region7 v3, v56
;; @0025                               trapz v53, user16
;;                                     v148 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v149 = load.i64 notrap aligned readonly can_move region6 v148+32
;; @0025                               v58 = uextend.i64 v53
;; @0025                               v61 = iadd v149, v58
;; @0025                               v63 = iadd v61, v55  ; v55 = 16
;; @0025                               v64 = load.i32 user2 readonly region7 v63
;; @0025                               v65 = uextend.i64 v64
;; @0025                               v71 = icmp.i64 ugt v8, v65
;; @0025                               trapnz v71, user17
;; @0025                               v82 = load.i32 notrap aligned region2 v0+56
;; @0025                               v83 = uextend.i64 v82
;; @0025                               v89 = icmp.i64 ugt v11, v83
;; @0025                               trapnz v89, heap_oob
;; @0025                               v90 = load.i64 notrap aligned region3 v0+48
;; @0025                               v101 = load.i64 notrap aligned region8 v148+40
;; @0025                               v76 = iconst.i64 20
;; @0025                               v77 = iadd v61, v76  ; v76 = 20
;; @0025                               v103 = uadd_overflow_trap v77, v8, user2
;; @0025                               v102 = iadd v149, v101
;; @0025                               v104 = icmp ugt v103, v102
;; @0025                               trapnz v104, user2
;; @0025                               v92 = iadd v90, v7
;; @0025                               call fn1(v0, v77, v92, v8), stack_map=[i32 @ ss0+0]
;;                                     v106 = load.i32 notrap v113
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v106
;; }
