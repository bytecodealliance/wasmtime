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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 56 "VMContext+0x38"
;;     region3 = 48 "VMContext+0x30"
;;     region4 = 32 "VMContext+0x20"
;;     region5 = 872415232 "VMCopyingHeapData+0x0"
;;     region6 = 872415236 "VMCopyingHeapData+0x4"
;;     region7 = 40 "VMContext+0x28"
;;     region8 = 1677721600 "TypeIdsArray+0x0"
;;     region9 = 67108896 "VMStoreContext+0x20"
;;     region10 = 536870912 "GcHeap"
;;     region11 = 67108904 "VMStoreContext+0x28"
;;     region12 = 1543503872 "Stack(ss0)"
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
;;                                     v121 = iconst.i32 15
;;                                     v122 = iadd.i32 v22, v121  ; v121 = 15
;;                                     v125 = iconst.i32 -16
;;                                     v126 = band v122, v125  ; v125 = -16
;;                                     v128 = iadd.i32 v24, v126
;; @0025                               store notrap aligned region5 v128, v23
;;                                     v142 = iconst.i32 -1476395002
;;                                     v143 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v144 = load.i64 notrap aligned readonly can_move region9 v143+32
;; @0025                               v48 = iadd v144, v31
;; @0025                               store user2 region10 v142, v48  ; v142 = -1476395002
;;                                     v145 = load.i64 notrap aligned readonly can_move region7 v0+40
;;                                     v146 = load.i32 notrap aligned readonly can_move region8 v145
;; @0025                               store user2 region10 v146, v48+4
;; @0025                               store user2 region10 v126, v48+8
;; @0025                               jump block4(v24, v48)
;;
;;                                 block3 cold:
;; @0025                               v35 = iconst.i32 -1476395002
;; @0025                               v36 = load.i64 notrap aligned readonly can_move region7 v0+40
;; @0025                               v37 = load.i32 notrap aligned readonly can_move region8 v36
;; @0025                               v38 = iconst.i32 16
;; @0025                               v39 = call fn0(v0, v35, v37, v22, v38)  ; v35 = -1476395002, v38 = 16
;; @0025                               v40 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v41 = load.i64 notrap aligned readonly can_move region9 v40+32
;; @0025                               v42 = uextend.i64 v39
;; @0025                               v43 = iadd v41, v42
;; @0025                               jump block4(v39, v43)
;;
;;                                 block4(v53: i32, v54: i64):
;;                                     v113 = stack_addr.i64 ss0
;;                                     store notrap aligned region12 v53, v113
;; @0025                               v55 = iconst.i64 16
;; @0025                               v56 = iadd v54, v55  ; v55 = 16
;; @0025                               store.i32 user2 region10 v3, v56
;; @0025                               trapz v53, user16
;;                                     v147 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v148 = load.i64 notrap aligned readonly can_move region9 v147+32
;; @0025                               v58 = uextend.i64 v53
;; @0025                               v61 = iadd v148, v58
;; @0025                               v63 = iadd v61, v55  ; v55 = 16
;; @0025                               v64 = load.i32 user2 readonly region10 v63
;; @0025                               v65 = uextend.i64 v64
;; @0025                               v71 = icmp.i64 ugt v7, v65
;; @0025                               trapnz v71, user17
;; @0025                               v82 = load.i32 notrap aligned region2 v0+56
;; @0025                               v83 = uextend.i64 v82
;; @0025                               v89 = icmp.i64 ugt v10, v83
;; @0025                               trapnz v89, heap_oob
;; @0025                               v90 = load.i64 notrap aligned region3 v0+48
;; @0025                               v101 = load.i64 notrap aligned region11 v147+40
;; @0025                               v76 = iconst.i64 20
;; @0025                               v77 = iadd v61, v76  ; v76 = 20
;; @0025                               v103 = uadd_overflow_trap v77, v7, user2
;; @0025                               v102 = iadd v148, v101
;; @0025                               v104 = icmp ugt v103, v102
;; @0025                               trapnz v104, user2
;; @0025                               v92 = iadd v90, v6
;; @0025                               call fn1(v0, v77, v92, v7), stack_map=[i32 @ ss0+0]
;; @0029                               jump block1
;;
;;                                 block1:
;;                                     v106 = load.i32 notrap aligned region12 v113
;; @0029                               return v106
;; }
