;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc -Ccollector=copying'

(module
  (type $a (array (mut anyref)))

  (func $fill-anything (param $a (ref $a)) (param $i i32) (param $v anyref) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )

  (func $fill-zero (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (ref.null any) (local.get $len))
  )

  (func $fill-pattern (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (ref.i31 (i32.const -1)) (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0030                               trapz v2, user16
;; @0030                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0030                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @0030                               v6 = uextend.i64 v2
;; @0030                               v9 = iadd v8, v6
;; @0030                               v10 = iconst.i64 16
;; @0030                               v11 = iadd v9, v10  ; v10 = 16
;; @0030                               v12 = load.i32 user2 readonly region4 v11
;; @0030                               v14 = uextend.i64 v3
;; @0030                               v15 = uextend.i64 v5
;; @0030                               v18 = iadd v14, v15
;; @0030                               v13 = uextend.i64 v12
;; @0030                               v19 = icmp ugt v18, v13
;; @0030                               trapnz v19, user17
;; @0030                               v36 = load.i64 notrap aligned region3 v7+40
;; @0030                               v24 = iconst.i64 20
;; @0030                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v49 = iconst.i64 2
;;                                     v50 = ishl v14, v49  ; v49 = 2
;; @0030                               v29 = iadd v25, v50
;;                                     v52 = ishl v15, v49  ; v49 = 2
;; @0030                               v38 = uadd_overflow_trap v29, v52, user2
;; @0030                               v37 = iadd v8, v36
;; @0030                               v39 = icmp ugt v38, v37
;; @0030                               trapnz v39, user2
;;                                     v47 = iconst.i64 0
;; @0030                               v42 = icmp eq v15, v47  ; v47 = 0
;; @0030                               v27 = iconst.i64 4
;; @0030                               v40 = iadd v29, v52
;; @0030                               brif v42, block3, block2(v29)
;;
;;                                 block2(v43: i64):
;; @0030                               store.i32 user2 little region4 v4, v43
;;                                     v54 = iconst.i64 4
;;                                     v55 = iadd v43, v54  ; v54 = 4
;; @0030                               v46 = icmp eq v55, v40
;; @0030                               brif v46, block3, block2(v55)
;;
;;                                 block3:
;; @0033                               jump block1
;;
;;                                 block1:
;; @0033                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @003e                               trapz v2, user16
;; @003e                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @003e                               v6 = uextend.i64 v2
;; @003e                               v9 = iadd v8, v6
;; @003e                               v10 = iconst.i64 16
;; @003e                               v11 = iadd v9, v10  ; v10 = 16
;; @003e                               v12 = load.i32 user2 readonly region4 v11
;; @003e                               v14 = uextend.i64 v3
;; @003e                               v15 = uextend.i64 v4
;; @003e                               v18 = iadd v14, v15
;; @003e                               v13 = uextend.i64 v12
;; @003e                               v19 = icmp ugt v18, v13
;; @003e                               trapnz v19, user17
;; @003e                               v36 = load.i64 notrap aligned region3 v7+40
;; @003e                               v24 = iconst.i64 20
;; @003e                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v49 = iconst.i64 2
;;                                     v50 = ishl v14, v49  ; v49 = 2
;; @003e                               v29 = iadd v25, v50
;;                                     v52 = ishl v15, v49  ; v49 = 2
;; @003e                               v38 = uadd_overflow_trap v29, v52, user2
;; @003e                               v37 = iadd v8, v36
;; @003e                               v39 = icmp ugt v38, v37
;; @003e                               trapnz v39, user2
;;                                     v47 = iconst.i64 0
;; @003e                               v42 = icmp eq v15, v47  ; v47 = 0
;; @003a                               v5 = iconst.i32 0
;; @003e                               v27 = iconst.i64 4
;; @003e                               v40 = iadd v29, v52
;; @003e                               brif v42, block3, block2(v29)
;;
;;                                 block2(v43: i64):
;;                                     v54 = iconst.i32 0
;; @003e                               store user2 little region4 v54, v43  ; v54 = 0
;;                                     v55 = iconst.i64 4
;;                                     v56 = iadd v43, v55  ; v55 = 4
;; @003e                               v46 = icmp eq v56, v40
;; @003e                               brif v46, block3, block2(v56)
;;
;;                                 block3:
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @004e                               trapz v2, user16
;; @004e                               v11 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004e                               v12 = load.i64 notrap aligned readonly can_move region2 v11+32
;; @004e                               v10 = uextend.i64 v2
;; @004e                               v13 = iadd v12, v10
;; @004e                               v14 = iconst.i64 16
;; @004e                               v15 = iadd v13, v14  ; v14 = 16
;; @004e                               v16 = load.i32 user2 readonly region4 v15
;; @004e                               v18 = uextend.i64 v3
;; @004e                               v19 = uextend.i64 v4
;; @004e                               v22 = iadd v18, v19
;; @004e                               v17 = uextend.i64 v16
;; @004e                               v23 = icmp ugt v22, v17
;; @004e                               trapnz v23, user17
;; @004e                               v40 = load.i64 notrap aligned region3 v11+40
;; @004e                               v28 = iconst.i64 20
;; @004e                               v29 = iadd v13, v28  ; v28 = 20
;;                                     v59 = iconst.i64 2
;;                                     v60 = ishl v18, v59  ; v59 = 2
;; @004e                               v33 = iadd v29, v60
;;                                     v62 = ishl v19, v59  ; v59 = 2
;; @004e                               v42 = uadd_overflow_trap v33, v62, user2
;; @004e                               v41 = iadd v12, v40
;; @004e                               v43 = icmp ugt v42, v41
;; @004e                               trapnz v43, user2
;;                                     v57 = iconst.i64 0
;; @004e                               v46 = icmp eq v19, v57  ; v57 = 0
;; @0048                               v5 = iconst.i32 -1
;; @004e                               v31 = iconst.i64 4
;; @004e                               v44 = iadd v33, v62
;; @004e                               brif v46, block3, block2(v33)
;;
;;                                 block2(v47: i64):
;;                                     v64 = iconst.i32 -1
;; @004e                               store user2 little region4 v64, v47  ; v64 = -1
;;                                     v65 = iconst.i64 4
;;                                     v66 = iadd v47, v65  ; v65 = 4
;; @004e                               v50 = icmp eq v66, v44
;; @004e                               brif v50, block3, block2(v66)
;;
;;                                 block3:
;; @0051                               jump block1
;;
;;                                 block1:
;; @0051                               return
;; }
