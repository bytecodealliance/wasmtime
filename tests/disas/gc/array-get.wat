;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty) i32) (result i64)
    (array.get $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0022                               trapz v2, user16
;; @0022                               v8 = uextend.i64 v2
;; @0022                               v9 = iconst.i64 16
;; @0022                               v10 = uadd_overflow_trap v8, v9, user1  ; v9 = 16
;; @0022                               v11 = iconst.i64 4
;; @0022                               v12 = uadd_overflow_trap v10, v11, user1  ; v11 = 4
;; @0022                               v7 = load.i64 notrap aligned readonly v0+48
;; @0022                               v13 = icmp ule v12, v7
;; @0022                               trapz v13, user1
;; @0022                               v6 = load.i64 notrap aligned readonly v0+40
;; @0022                               v14 = iadd v6, v10
;; @0022                               v15 = load.i32 notrap aligned v14
;; @0022                               v16 = icmp ult v3, v15
;; @0022                               trapz v16, user17
;; @0022                               v18 = uextend.i64 v15
;;                                     v39 = iconst.i64 3
;;                                     v40 = ishl v18, v39  ; v39 = 3
;;                                     v38 = iconst.i64 32
;; @0022                               v20 = ushr v40, v38  ; v38 = 32
;; @0022                               trapnz v20, user1
;;                                     v49 = iconst.i32 3
;;                                     v50 = ishl v15, v49  ; v49 = 3
;; @0022                               v22 = iconst.i32 24
;; @0022                               v23 = uadd_overflow_trap v50, v22, user1  ; v22 = 24
;;                                     v57 = ishl v3, v49  ; v49 = 3
;; @0022                               v26 = iadd v57, v22  ; v22 = 24
;; @0022                               v31 = uextend.i64 v26
;; @0022                               v32 = uadd_overflow_trap v8, v31, user1
;; @0022                               v33 = uextend.i64 v23
;; @0022                               v34 = uadd_overflow_trap v8, v33, user1
;; @0022                               v35 = icmp ule v34, v7
;; @0022                               trapz v35, user1
;; @0022                               v36 = iadd v6, v32
;; @0022                               v37 = load.i64 notrap aligned little v36
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v37
;; }
