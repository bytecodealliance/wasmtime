;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut i8)))

  (func $fill (param $a (ref $a)) (param $i i32) (param $v i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32) tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0027                               trapz v2, user16
;; @0027                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0027                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @0027                               v6 = uextend.i64 v2
;; @0027                               v9 = iadd v8, v6
;; @0027                               v10 = iconst.i64 16
;; @0027                               v11 = iadd v9, v10  ; v10 = 16
;; @0027                               v12 = load.i32 user2 readonly region4 v11
;; @0027                               v14 = uextend.i64 v3
;; @0027                               v15 = uextend.i64 v5
;; @0027                               v18 = iadd v14, v15
;; @0027                               v13 = uextend.i64 v12
;; @0027                               v19 = icmp ugt v18, v13
;; @0027                               trapnz v19, user17
;; @0027                               v36 = load.i64 notrap aligned region3 v7+40
;; @0027                               v24 = iconst.i64 20
;; @0027                               v25 = iadd v9, v24  ; v24 = 20
;; @0027                               v29 = iadd v25, v14
;; @0027                               v38 = uadd_overflow_trap v29, v15, user2
;; @0027                               v37 = iadd v8, v36
;; @0027                               v39 = icmp ugt v38, v37
;; @0027                               trapnz v39, user2
;; @0027                               call fn0(v0, v29, v4, v15)
;; @002a                               jump block1
;;
;;                                 block1:
;; @002a                               return
;; }
