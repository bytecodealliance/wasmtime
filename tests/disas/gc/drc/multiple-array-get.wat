;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty) i32 i32) (result i64 i64)
    (array.get $ty (local.get 0) (local.get 1))
    (array.get $ty (local.get 0) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) -> i64, i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0024                               trapz v2, user16
;; @0024                               v65 = load.i64 notrap aligned readonly can_move v0+8
;; @0024                               v8 = load.i64 notrap aligned readonly can_move v65+24
;; @0024                               v7 = uextend.i64 v2
;; @0024                               v9 = iadd v8, v7
;; @0024                               v10 = iconst.i64 24
;; @0024                               v11 = iadd v9, v10  ; v10 = 24
;; @0024                               v12 = load.i32 notrap aligned readonly v11
;; @0024                               v13 = icmp ult v3, v12
;; @0024                               trapz v13, user17
;; @0024                               v15 = uextend.i64 v12
;;                                     v67 = iconst.i64 3
;;                                     v68 = ishl v15, v67  ; v67 = 3
;;                                     v64 = iconst.i64 32
;; @0024                               v17 = ushr v68, v64  ; v64 = 32
;; @0024                               trapnz v17, user1
;;                                     v77 = iconst.i32 3
;;                                     v78 = ishl v12, v77  ; v77 = 3
;; @0024                               v19 = iconst.i32 32
;; @0024                               v20 = uadd_overflow_trap v78, v19, user1  ; v19 = 32
;; @0024                               v24 = uadd_overflow_trap v2, v20, user1
;; @0024                               v25 = uextend.i64 v24
;; @0024                               v27 = iadd v8, v25
;;                                     v84 = ishl v3, v77  ; v77 = 3
;; @0024                               v23 = iadd v84, v19  ; v19 = 32
;; @0024                               v28 = isub v20, v23
;; @0024                               v29 = uextend.i64 v28
;; @0024                               v30 = isub v27, v29
;; @0024                               v31 = load.i64 notrap aligned little v30
;; @002b                               v38 = icmp ult v4, v12
;; @002b                               trapz v38, user17
;;                                     v86 = ishl v4, v77  ; v77 = 3
;; @002b                               v48 = iadd v86, v19  ; v19 = 32
;; @002b                               v53 = isub v20, v48
;; @002b                               v54 = uextend.i64 v53
;; @002b                               v55 = isub v27, v54
;; @002b                               v56 = load.i64 notrap aligned little v55
;; @002e                               jump block1
;;
;;                                 block1:
;; @002e                               return v31, v56
;; }
