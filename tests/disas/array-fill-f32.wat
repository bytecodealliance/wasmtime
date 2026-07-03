;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut f32)))

  (func $fill-anything (param $a (ref $a)) (param $i i32) (param $v f32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )

  (func $fill-zero (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (f32.const 0) (local.get $len))
  )

  (func $fill-bit-pattern (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (f32.const 1) (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, f32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108896 "VMStoreContext+0x20"
;;     region3 = 67108904 "VMStoreContext+0x28"
;;     region4 = 536870912 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: f32, v5: i32):
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
;; @0030                               store.f32 user2 little region4 v4, v43
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
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108896 "VMStoreContext+0x20"
;;     region3 = 67108904 "VMStoreContext+0x28"
;;     region4 = 536870912 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0041                               trapz v2, user16
;; @0041                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0041                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @0041                               v6 = uextend.i64 v2
;; @0041                               v9 = iadd v8, v6
;; @0041                               v10 = iconst.i64 16
;; @0041                               v11 = iadd v9, v10  ; v10 = 16
;; @0041                               v12 = load.i32 user2 readonly region4 v11
;; @0041                               v14 = uextend.i64 v3
;; @0041                               v15 = uextend.i64 v4
;; @0041                               v18 = iadd v14, v15
;; @0041                               v13 = uextend.i64 v12
;; @0041                               v19 = icmp ugt v18, v13
;; @0041                               trapnz v19, user17
;; @0041                               v36 = load.i64 notrap aligned region3 v7+40
;; @0041                               v24 = iconst.i64 20
;; @0041                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v43 = iconst.i64 2
;;                                     v44 = ishl v14, v43  ; v43 = 2
;; @0041                               v29 = iadd v25, v44
;;                                     v46 = ishl v15, v43  ; v43 = 2
;; @0041                               v38 = uadd_overflow_trap v29, v46, user2
;; @0041                               v37 = iadd v8, v36
;; @0041                               v39 = icmp ugt v38, v37
;; @0041                               trapnz v39, user2
;; @0041                               v40 = iconst.i32 0
;; @0041                               call fn0(v0, v29, v40, v46)  ; v40 = 0
;; @0044                               jump block1
;;
;;                                 block1:
;; @0044                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108896 "VMStoreContext+0x20"
;;     region3 = 67108904 "VMStoreContext+0x28"
;;     region4 = 536870912 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0052                               trapz v2, user16
;; @0052                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0052                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @0052                               v6 = uextend.i64 v2
;; @0052                               v9 = iadd v8, v6
;; @0052                               v10 = iconst.i64 16
;; @0052                               v11 = iadd v9, v10  ; v10 = 16
;; @0052                               v12 = load.i32 user2 readonly region4 v11
;; @0052                               v14 = uextend.i64 v3
;; @0052                               v15 = uextend.i64 v4
;; @0052                               v18 = iadd v14, v15
;; @0052                               v13 = uextend.i64 v12
;; @0052                               v19 = icmp ugt v18, v13
;; @0052                               trapnz v19, user17
;; @0052                               v36 = load.i64 notrap aligned region3 v7+40
;; @0052                               v24 = iconst.i64 20
;; @0052                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v49 = iconst.i64 2
;;                                     v50 = ishl v14, v49  ; v49 = 2
;; @0052                               v29 = iadd v25, v50
;;                                     v52 = ishl v15, v49  ; v49 = 2
;; @0052                               v38 = uadd_overflow_trap v29, v52, user2
;; @0052                               v37 = iadd v8, v36
;; @0052                               v39 = icmp ugt v38, v37
;; @0052                               trapnz v39, user2
;;                                     v47 = iconst.i64 0
;; @0052                               v42 = icmp eq v15, v47  ; v47 = 0
;; @004b                               v5 = f32const 0x1.000000p0
;; @0052                               v27 = iconst.i64 4
;; @0052                               v40 = iadd v29, v52
;; @0052                               brif v42, block3, block2(v29)
;;
;;                                 block2(v43: i64):
;;                                     v54 = f32const 0x1.000000p0
;; @0052                               store user2 little region4 v54, v43  ; v54 = 0x1.000000p0
;;                                     v55 = iconst.i64 4
;;                                     v56 = iadd v43, v55  ; v55 = 4
;; @0052                               v46 = icmp eq v56, v40
;; @0052                               brif v46, block3, block2(v56)
;;
;;                                 block3:
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return
;; }
