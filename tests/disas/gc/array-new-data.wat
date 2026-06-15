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
;;     region5 = 3489660928 "VMCopyingHeapData+0x0"
;;     region6 = 3489660932 "VMCopyingHeapData+0x4"
;;     region7 = 40 "VMContext+0x28"
;;     region8 = 268435488 "VMStoreContext+0x20"
;;     region9 = 2147483648 "GcHeap"
;;     region10 = 268435496 "VMStoreContext+0x28"
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
;; @0025                               v4 = load.i32 notrap aligned region2 v0+56
;; @0025                               v6 = uextend.i64 v2
;; @0025                               v7 = uextend.i64 v3
;; @0025                               v10 = iadd v6, v7
;; @0025                               v5 = uextend.i64 v4
;; @0025                               v11 = icmp ugt v10, v5
;; @0025                               trapnz v11, heap_oob
;; @0025                               v12 = load.i64 notrap aligned region3 v0+48
;; @0025                               v19 = iconst.i64 32
;; @0025                               v20 = ushr v7, v19  ; v19 = 32
;; @0025                               trapnz v20, user18
;; @0025                               v15 = iconst.i32 20
;; @0025                               v22 = uadd_overflow_trap v15, v3, user18  ; v15 = 20
;; @0025                               v23 = load.i64 notrap aligned readonly can_move region4 v0+32
;; @0025                               v24 = load.i32 notrap aligned region5 v23
;; @0025                               v25 = load.i32 notrap aligned region6 v23+4
;; @0025                               v31 = uextend.i64 v24
;; @0025                               v26 = uextend.i64 v22
;; @0025                               v27 = iconst.i64 15
;; @0025                               v29 = iadd v26, v27  ; v27 = 15
;; @0025                               v28 = iconst.i64 -16
;; @0025                               v30 = band v29, v28  ; v28 = -16
;; @0025                               v32 = iadd v31, v30
;; @0025                               v33 = uextend.i64 v25
;; @0025                               v34 = icmp ule v32, v33
;; @0025                               brif v34, block2, block3
;;
;;                                 block2:
;;                                     v120 = iconst.i32 15
;;                                     v121 = iadd.i32 v22, v120  ; v120 = 15
;;                                     v124 = iconst.i32 -16
;;                                     v125 = band v121, v124  ; v124 = -16
;;                                     v127 = iadd.i32 v24, v125
;; @0025                               store notrap aligned region5 v127, v23
;;                                     v141 = iconst.i32 -1476395002
;;                                     v142 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v143 = load.i64 notrap aligned readonly can_move region8 v142+32
;; @0025                               v48 = iadd v143, v31
;; @0025                               store user2 region9 v141, v48  ; v141 = -1476395002
;;                                     v144 = load.i64 notrap aligned readonly can_move region7 v0+40
;;                                     v145 = load.i32 notrap aligned readonly can_move v144
;; @0025                               store user2 region9 v145, v48+4
;;                                     v146 = band.i64 v29, v28  ; v28 = -16
;; @0025                               istore32 user2 region9 v146, v48+8
;; @0025                               jump block4(v24, v48)
;;
;;                                 block3 cold:
;; @0025                               v35 = iconst.i32 -1476395002
;; @0025                               v36 = load.i64 notrap aligned readonly can_move region7 v0+40
;; @0025                               v37 = load.i32 notrap aligned readonly can_move v36
;; @0025                               v38 = iconst.i32 16
;; @0025                               v39 = call fn0(v0, v35, v37, v22, v38)  ; v35 = -1476395002, v38 = 16
;; @0025                               v40 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v41 = load.i64 notrap aligned readonly can_move region8 v40+32
;; @0025                               v42 = uextend.i64 v39
;; @0025                               v43 = iadd v41, v42
;; @0025                               jump block4(v39, v43)
;;
;;                                 block4(v52: i32, v53: i64):
;;                                     v112 = stack_addr.i64 ss0
;;                                     store notrap v52, v112
;; @0025                               v54 = iconst.i64 16
;; @0025                               v55 = iadd v53, v54  ; v54 = 16
;; @0025                               store.i32 user2 region9 v3, v55
;; @0025                               trapz v52, user16
;;                                     v147 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v148 = load.i64 notrap aligned readonly can_move region8 v147+32
;; @0025                               v57 = uextend.i64 v52
;; @0025                               v60 = iadd v148, v57
;; @0025                               v62 = iadd v60, v54  ; v54 = 16
;; @0025                               v63 = load.i32 user2 readonly region9 v62
;; @0025                               v64 = uextend.i64 v63
;; @0025                               v70 = icmp.i64 ugt v7, v64
;; @0025                               trapnz v70, user17
;; @0025                               v81 = load.i32 notrap aligned region2 v0+56
;; @0025                               v82 = uextend.i64 v81
;; @0025                               v88 = icmp.i64 ugt v10, v82
;; @0025                               trapnz v88, heap_oob
;; @0025                               v89 = load.i64 notrap aligned region3 v0+48
;; @0025                               v100 = load.i64 notrap aligned region10 v147+40
;; @0025                               v75 = iconst.i64 20
;; @0025                               v76 = iadd v60, v75  ; v75 = 20
;; @0025                               v102 = uadd_overflow_trap v76, v7, user2
;; @0025                               v101 = iadd v148, v100
;; @0025                               v103 = icmp ugt v102, v101
;; @0025                               trapnz v103, user2
;; @0025                               v91 = iadd v89, v6
;; @0025                               call fn1(v0, v76, v91, v7), stack_map=[i32 @ ss0+0]
;; @0029                               jump block1
;;
;;                                 block1:
;;                                     v105 = load.i32 notrap v112
;; @0029                               return v105
;; }
