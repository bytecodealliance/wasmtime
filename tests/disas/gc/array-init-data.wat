;;! target = "x86_64"
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (data $passive "this is a passive data segment")
  (type $a (array (mut i8)))

  (func $a (param (ref $a) i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    array.init_data $a $passive)
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108896 "VMStoreContext+0x20"
;;     region3 = 67108904 "VMStoreContext+0x28"
;;     region4 = 536870912 "GcHeap"
;;     region5 = 56 "VMContext+0x38"
;;     region6 = 48 "VMContext+0x30"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:1 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @002a                               trapz v2, user16
;; @002a                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002a                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @002a                               v6 = uextend.i64 v2
;; @002a                               v9 = iadd v8, v6
;; @002a                               v10 = iconst.i64 16
;; @002a                               v11 = iadd v9, v10  ; v10 = 16
;; @002a                               v12 = load.i32 user2 readonly region4 v11
;; @002a                               v14 = uextend.i64 v3
;; @002a                               v15 = uextend.i64 v5
;; @002a                               v18 = iadd v14, v15
;; @002a                               v13 = uextend.i64 v12
;; @002a                               v19 = icmp ugt v18, v13
;; @002a                               trapnz v19, user17
;; @002a                               v30 = load.i32 notrap aligned region5 v0+56
;; @002a                               v32 = uextend.i64 v4
;; @002a                               v36 = iadd v32, v15
;; @002a                               v31 = uextend.i64 v30
;; @002a                               v37 = icmp ugt v36, v31
;; @002a                               trapnz v37, heap_oob
;; @002a                               v38 = load.i64 notrap aligned region6 v0+48
;; @002a                               v49 = load.i64 notrap aligned region3 v7+40
;; @002a                               v24 = iconst.i64 20
;; @002a                               v25 = iadd v9, v24  ; v24 = 20
;; @002a                               v29 = iadd v25, v14
;; @002a                               v51 = uadd_overflow_trap v29, v15, user2
;; @002a                               v50 = iadd v8, v49
;; @002a                               v52 = icmp ugt v51, v50
;; @002a                               trapnz v52, user2
;; @002a                               v40 = iadd v38, v32
;; @002a                               call fn0(v0, v29, v40, v15)
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return
;; }
