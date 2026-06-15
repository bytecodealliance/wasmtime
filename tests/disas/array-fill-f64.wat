;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut f64)))

  (func $fill-anything (param $a (ref $a)) (param $i i32) (param $v f64) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )

  (func $fill-zero (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (f64.const 0) (local.get $len))
  )

  (func $fill-bit-pattern (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (f64.const 1) (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, f64, i32) tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: f64, v5: i32):
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
;; @0030                               v24 = iconst.i64 24
;; @0030                               v25 = iadd v9, v24  ; v24 = 24
;;                                     v49 = iconst.i64 3
;;                                     v50 = ishl v14, v49  ; v49 = 3
;; @0030                               v29 = iadd v25, v50
;;                                     v52 = ishl v15, v49  ; v49 = 3
;; @0030                               v38 = uadd_overflow_trap v29, v52, user2
;; @0030                               v37 = iadd v8, v36
;; @0030                               v39 = icmp ugt v38, v37
;; @0030                               trapnz v39, user2
;;                                     v47 = iconst.i64 0
;; @0030                               v42 = icmp eq v15, v47  ; v47 = 0
;; @0030                               v27 = iconst.i64 8
;; @0030                               v40 = iadd v29, v52
;; @0030                               brif v42, block3, block2(v29)
;;
;;                                 block2(v43: i64):
;; @0030                               store.f64 user2 little region4 v4, v43
;;                                     v54 = iconst.i64 8
;;                                     v55 = iadd v43, v54  ; v54 = 8
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
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0045                               trapz v2, user16
;; @0045                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0045                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @0045                               v6 = uextend.i64 v2
;; @0045                               v9 = iadd v8, v6
;; @0045                               v10 = iconst.i64 16
;; @0045                               v11 = iadd v9, v10  ; v10 = 16
;; @0045                               v12 = load.i32 user2 readonly region4 v11
;; @0045                               v14 = uextend.i64 v3
;; @0045                               v15 = uextend.i64 v4
;; @0045                               v18 = iadd v14, v15
;; @0045                               v13 = uextend.i64 v12
;; @0045                               v19 = icmp ugt v18, v13
;; @0045                               trapnz v19, user17
;; @0045                               v36 = load.i64 notrap aligned region3 v7+40
;; @0045                               v24 = iconst.i64 24
;; @0045                               v25 = iadd v9, v24  ; v24 = 24
;;                                     v43 = iconst.i64 3
;;                                     v44 = ishl v14, v43  ; v43 = 3
;; @0045                               v29 = iadd v25, v44
;;                                     v46 = ishl v15, v43  ; v43 = 3
;; @0045                               v38 = uadd_overflow_trap v29, v46, user2
;; @0045                               v37 = iadd v8, v36
;; @0045                               v39 = icmp ugt v38, v37
;; @0045                               trapnz v39, user2
;; @0045                               v40 = iconst.i32 0
;; @0045                               call fn0(v0, v29, v40, v46)  ; v40 = 0
;; @0048                               jump block1
;;
;;                                 block1:
;; @0048                               return
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
;; @005a                               trapz v2, user16
;; @005a                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @005a                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @005a                               v6 = uextend.i64 v2
;; @005a                               v9 = iadd v8, v6
;; @005a                               v10 = iconst.i64 16
;; @005a                               v11 = iadd v9, v10  ; v10 = 16
;; @005a                               v12 = load.i32 user2 readonly region4 v11
;; @005a                               v14 = uextend.i64 v3
;; @005a                               v15 = uextend.i64 v4
;; @005a                               v18 = iadd v14, v15
;; @005a                               v13 = uextend.i64 v12
;; @005a                               v19 = icmp ugt v18, v13
;; @005a                               trapnz v19, user17
;; @005a                               v36 = load.i64 notrap aligned region3 v7+40
;; @005a                               v24 = iconst.i64 24
;; @005a                               v25 = iadd v9, v24  ; v24 = 24
;;                                     v49 = iconst.i64 3
;;                                     v50 = ishl v14, v49  ; v49 = 3
;; @005a                               v29 = iadd v25, v50
;;                                     v52 = ishl v15, v49  ; v49 = 3
;; @005a                               v38 = uadd_overflow_trap v29, v52, user2
;; @005a                               v37 = iadd v8, v36
;; @005a                               v39 = icmp ugt v38, v37
;; @005a                               trapnz v39, user2
;;                                     v47 = iconst.i64 0
;; @005a                               v42 = icmp eq v15, v47  ; v47 = 0
;; @004f                               v5 = f64const 0x1.0000000000000p0
;; @005a                               v27 = iconst.i64 8
;; @005a                               v40 = iadd v29, v52
;; @005a                               brif v42, block3, block2(v29)
;;
;;                                 block2(v43: i64):
;;                                     v54 = f64const 0x1.0000000000000p0
;; @005a                               store user2 little region4 v54, v43  ; v54 = 0x1.0000000000000p0
;;                                     v55 = iconst.i64 8
;;                                     v56 = iadd v43, v55  ; v55 = 8
;; @005a                               v46 = icmp eq v56, v40
;; @005a                               brif v46, block3, block2(v56)
;;
;;                                 block3:
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return
;; }
