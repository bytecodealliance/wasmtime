;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc -O gc-heap-may-move=n -O gc-heap-reservation=131072"
;;! test = "optimize"

;; With a small GC heap reservation, array element accesses are explicitly
;; bounds checked. The field's address is computed in full before the Spectre
;; guard, so an out-of-bounds access loads from NULL.

(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty) i32) (result i64)
    (array.get $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i64 tail {
;;     region0 = 123 ""
;;     region1 = 25 ""
;;     region2 = 160 ""
;;     region3 = 196 ""
;;     region4 = 108 ""
;;     region5 = 5 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0022                               trapz v2, user16
;; @0022                               v4 = uextend.i64 v2
;; @0022                               v5 = iconst.i64 0x0001_ffe4
;; @0022                               v6 = icmp ugt v4, v5  ; v5 = 0x0001_ffe4
;; @0022                               v12 = iconst.i64 0
;; @0022                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v8 = load.i64 notrap aligned readonly can_move region2 v7+24
;; @0022                               v9 = iadd v8, v4
;; @0022                               v10 = iconst.i64 24
;; @0022                               v11 = iadd v9, v10  ; v10 = 24
;; @0022                               v13 = select_spectre_guard v6, v12, v11  ; v12 = 0
;; @0022                               v14 = load.i32 user2 readonly region4 v13
;; @0022                               v15 = icmp ult v3, v14
;; @0022                               trapz v15, user17
;; @0022                               v17 = uextend.i64 v14
;;                                     v41 = iconst.i64 3
;;                                     v42 = ishl v17, v41  ; v41 = 3
;; @0022                               v19 = iconst.i64 32
;; @0022                               v20 = ushr v42, v19  ; v19 = 32
;; @0022                               trapnz v20, user2
;;                                     v49 = iconst.i32 3
;;                                     v50 = ishl v14, v49  ; v49 = 3
;; @0022                               v22 = iconst.i32 32
;; @0022                               v23 = uadd_overflow_trap v50, v22, user2  ; v22 = 32
;; @0022                               v27 = uadd_overflow_trap v2, v23, user2
;; @0022                               v30 = uextend.i64 v27
;; @0022                               v31 = iconst.i64 0x0002_0000
;; @0022                               v32 = icmp ugt v30, v31  ; v31 = 0x0002_0000
;; @0022                               v35 = iadd v8, v30
;;                                     v55 = ishl v3, v49  ; v49 = 3
;; @0022                               v26 = iadd v55, v22  ; v22 = 32
;; @0022                               v28 = isub v23, v26
;; @0022                               v29 = uextend.i64 v28
;; @0022                               v36 = isub v35, v29
;; @0022                               v38 = select_spectre_guard v32, v12, v36  ; v12 = 0
;; @0022                               v39 = load.i64 user2 little region5 v38
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v39
;; }
