;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty) i32 i64)
    (array.set $ty (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64):
;; @0024                               trapz v2, user16
;; @0024                               v9 = uextend.i64 v2
;; @0024                               v10 = iconst.i64 16
;; @0024                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 16
;; @0024                               v12 = iconst.i64 4
;; @0024                               v13 = uadd_overflow_trap v11, v12, user1  ; v12 = 4
;; @0024                               v8 = load.i64 notrap aligned readonly v0+48
;; @0024                               v14 = icmp ule v13, v8
;; @0024                               trapz v14, user1
;; @0024                               v6 = load.i64 notrap aligned readonly v0+40
;; @0024                               v15 = iadd v6, v11
;; @0024                               v16 = load.i32 notrap aligned v15
;; @0024                               v17 = icmp ult v3, v16
;; @0024                               trapz v17, user17
;; @0024                               v19 = uextend.i64 v16
;;                                     v40 = iconst.i64 3
;;                                     v41 = ishl v19, v40  ; v40 = 3
;;                                     v39 = iconst.i64 32
;; @0024                               v21 = ushr v41, v39  ; v39 = 32
;; @0024                               trapnz v21, user1
;;                                     v50 = iconst.i32 3
;;                                     v51 = ishl v16, v50  ; v50 = 3
;; @0024                               v23 = iconst.i32 24
;; @0024                               v24 = uadd_overflow_trap v51, v23, user1  ; v23 = 24
;;                                     v58 = ishl v3, v50  ; v50 = 3
;; @0024                               v27 = iadd v58, v23  ; v23 = 24
;; @0024                               v33 = uextend.i64 v27
;; @0024                               v34 = uadd_overflow_trap v9, v33, user1
;; @0024                               v35 = uextend.i64 v24
;; @0024                               v36 = uadd_overflow_trap v9, v35, user1
;; @0024                               v37 = icmp ule v36, v8
;; @0024                               trapz v37, user1
;; @0024                               v38 = iadd v6, v34
;; @0024                               store notrap aligned little v4, v38
;; @0027                               jump block1
;;
;;                                 block1:
;; @0027                               return
;; }
