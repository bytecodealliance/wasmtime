;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc -Ccollector=copying'

(module
  (type $a (array (mut externref)))

  (func $fill-anything (param $a (ref $a)) (param $i i32) (param $v externref) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )

  (func $fill-zero (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (ref.null extern) (local.get $len))
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
;; @002f                               trapz v2, user16
;; @002f                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002f                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @002f                               v6 = uextend.i64 v2
;; @002f                               v9 = iadd v8, v6
;; @002f                               v10 = iconst.i64 16
;; @002f                               v11 = iadd v9, v10  ; v10 = 16
;; @002f                               v12 = load.i32 user2 readonly region4 v11
;; @002f                               v14 = uextend.i64 v3
;; @002f                               v15 = uextend.i64 v5
;; @002f                               v18 = iadd v14, v15
;; @002f                               v13 = uextend.i64 v12
;; @002f                               v19 = icmp ugt v18, v13
;; @002f                               trapnz v19, user17
;; @002f                               v36 = load.i64 notrap aligned region3 v7+40
;; @002f                               v24 = iconst.i64 20
;; @002f                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v49 = iconst.i64 2
;;                                     v50 = ishl v14, v49  ; v49 = 2
;; @002f                               v29 = iadd v25, v50
;;                                     v52 = ishl v15, v49  ; v49 = 2
;; @002f                               v38 = uadd_overflow_trap v29, v52, user2
;; @002f                               v37 = iadd v8, v36
;; @002f                               v39 = icmp ugt v38, v37
;; @002f                               trapnz v39, user2
;;                                     v47 = iconst.i64 0
;; @002f                               v42 = icmp eq v15, v47  ; v47 = 0
;; @002f                               v27 = iconst.i64 4
;; @002f                               v40 = iadd v29, v52
;; @002f                               brif v42, block3, block2(v29)
;;
;;                                 block2(v43: i64):
;; @002f                               store.i32 user2 little region4 v4, v43
;;                                     v54 = iconst.i64 4
;;                                     v55 = iadd v43, v54  ; v54 = 4
;; @002f                               v46 = icmp eq v55, v40
;; @002f                               brif v46, block3, block2(v55)
;;
;;                                 block3:
;; @0032                               jump block1
;;
;;                                 block1:
;; @0032                               return
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
;; @003d                               trapz v2, user16
;; @003d                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003d                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @003d                               v6 = uextend.i64 v2
;; @003d                               v9 = iadd v8, v6
;; @003d                               v10 = iconst.i64 16
;; @003d                               v11 = iadd v9, v10  ; v10 = 16
;; @003d                               v12 = load.i32 user2 readonly region4 v11
;; @003d                               v14 = uextend.i64 v3
;; @003d                               v15 = uextend.i64 v4
;; @003d                               v18 = iadd v14, v15
;; @003d                               v13 = uextend.i64 v12
;; @003d                               v19 = icmp ugt v18, v13
;; @003d                               trapnz v19, user17
;; @003d                               v36 = load.i64 notrap aligned region3 v7+40
;; @003d                               v24 = iconst.i64 20
;; @003d                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v49 = iconst.i64 2
;;                                     v50 = ishl v14, v49  ; v49 = 2
;; @003d                               v29 = iadd v25, v50
;;                                     v52 = ishl v15, v49  ; v49 = 2
;; @003d                               v38 = uadd_overflow_trap v29, v52, user2
;; @003d                               v37 = iadd v8, v36
;; @003d                               v39 = icmp ugt v38, v37
;; @003d                               trapnz v39, user2
;;                                     v47 = iconst.i64 0
;; @003d                               v42 = icmp eq v15, v47  ; v47 = 0
;; @0039                               v5 = iconst.i32 0
;; @003d                               v27 = iconst.i64 4
;; @003d                               v40 = iadd v29, v52
;; @003d                               brif v42, block3, block2(v29)
;;
;;                                 block2(v43: i64):
;;                                     v54 = iconst.i32 0
;; @003d                               store user2 little region4 v54, v43  ; v54 = 0
;;                                     v55 = iconst.i64 4
;;                                     v56 = iadd v43, v55  ; v55 = 4
;; @003d                               v46 = icmp eq v56, v40
;; @003d                               brif v46, block3, block2(v56)
;;
;;                                 block3:
;; @0040                               jump block1
;;
;;                                 block1:
;; @0040                               return
;; }
