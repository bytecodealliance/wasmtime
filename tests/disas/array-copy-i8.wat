;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut i8)))

  (func $copy (param (ref $a) i32 (ref $a) i32 i32)
    (array.copy $a $a (local.get 0) (local.get 1) (local.get 2) (local.get 3) (local.get 4))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:1 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32):
;; @002b                               trapz v2, user16
;; @002b                               v8 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002b                               v9 = load.i64 notrap aligned readonly can_move region2 v8+32
;; @002b                               v7 = uextend.i64 v2
;; @002b                               v10 = iadd v9, v7
;; @002b                               v11 = iconst.i64 16
;; @002b                               v12 = iadd v10, v11  ; v11 = 16
;; @002b                               v13 = load.i32 user2 readonly region4 v12
;; @002b                               v15 = uextend.i64 v3
;; @002b                               v16 = uextend.i64 v6
;; @002b                               v19 = iadd v15, v16
;; @002b                               v14 = uextend.i64 v13
;; @002b                               v20 = icmp ugt v19, v14
;; @002b                               trapnz v20, user17
;; @002b                               trapz v4, user16
;; @002b                               v31 = uextend.i64 v4
;; @002b                               v34 = iadd v9, v31
;; @002b                               v36 = iadd v34, v11  ; v11 = 16
;; @002b                               v37 = load.i32 user2 readonly region4 v36
;; @002b                               v39 = uextend.i64 v5
;; @002b                               v43 = iadd v39, v16
;; @002b                               v38 = uextend.i64 v37
;; @002b                               v44 = icmp ugt v43, v38
;; @002b                               trapnz v44, user17
;; @002b                               v63 = load.i64 notrap aligned region3 v8+40
;; @002b                               v25 = iconst.i64 20
;; @002b                               v26 = iadd v10, v25  ; v25 = 20
;; @002b                               v30 = iadd v26, v15
;; @002b                               v65 = uadd_overflow_trap v30, v16, user2
;; @002b                               v64 = iadd v9, v63
;; @002b                               v66 = icmp ugt v65, v64
;; @002b                               trapnz v66, user2
;; @002b                               v50 = iadd v34, v25  ; v25 = 20
;; @002b                               v54 = iadd v50, v39
;; @002b                               v72 = uadd_overflow_trap v54, v16, user2
;; @002b                               v73 = icmp ugt v72, v64
;; @002b                               trapnz v73, user2
;; @002b                               call fn0(v0, v30, v54, v16)
;; @002f                               jump block1
;;
;;                                 block1:
;; @002f                               return
;; }
