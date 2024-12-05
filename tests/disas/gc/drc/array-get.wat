;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0022                               trapz v2, user16
;; @0022                               v9 = uextend.i64 v2
;; @0022                               v10 = iconst.i64 16
;; @0022                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 16
;; @0022                               v12 = iconst.i64 4
;; @0022                               v13 = uadd_overflow_trap v11, v12, user1  ; v12 = 4
;; @0022                               v8 = load.i64 notrap aligned readonly v0+48
;; @0022                               v14 = icmp ule v13, v8
;; @0022                               trapz v14, user1
;; @0022                               v6 = load.i64 notrap aligned readonly v0+40
;; @0022                               v15 = iadd v6, v11
;; @0022                               v16 = load.i32 notrap aligned v15
;; @0022                               v17 = icmp ult v3, v16
;; @0022                               trapz v17, user17
;; @0022                               v19 = uextend.i64 v16
;;                                     v41 = iconst.i64 3
;;                                     v42 = ishl v19, v41  ; v41 = 3
;;                                     v40 = iconst.i64 32
;; @0022                               v21 = ushr v42, v40  ; v40 = 32
;; @0022                               trapnz v21, user1
;;                                     v51 = iconst.i32 3
;;                                     v52 = ishl v16, v51  ; v51 = 3
;; @0022                               v23 = iconst.i32 24
;; @0022                               v24 = uadd_overflow_trap v52, v23, user1  ; v23 = 24
;;                                     v59 = ishl v3, v51  ; v51 = 3
;; @0022                               v27 = iadd v59, v23  ; v23 = 24
;; @0022                               v33 = uextend.i64 v27
;; @0022                               v34 = uadd_overflow_trap v9, v33, user1
;; @0022                               v35 = uextend.i64 v24
;; @0022                               v36 = uadd_overflow_trap v9, v35, user1
;; @0022                               v37 = icmp ule v36, v8
;; @0022                               trapz v37, user1
;; @0022                               v38 = iadd v6, v34
;; @0022                               v39 = load.i64 notrap aligned little v38
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v39
;; }
