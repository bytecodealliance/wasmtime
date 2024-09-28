;;! target = "x86_64"
;;! flags = "-W function-references,gc"
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
;; @0024                               v8 = uextend.i64 v2
;; @0024                               v9 = iconst.i64 16
;; @0024                               v10 = uadd_overflow_trap v8, v9, user1  ; v9 = 16
;; @0024                               v11 = iconst.i64 4
;; @0024                               v12 = uadd_overflow_trap v10, v11, user1  ; v11 = 4
;; @0024                               v7 = load.i64 notrap aligned readonly v0+48
;; @0024                               v13 = icmp ule v12, v7
;; @0024                               trapz v13, user1
;; @0024                               v6 = load.i64 notrap aligned readonly v0+40
;; @0024                               v14 = iadd v6, v10
;; @0024                               v15 = load.i32 notrap aligned v14
;; @0024                               v16 = icmp ult v3, v15
;; @0024                               trapz v16, user17
;; @0024                               v18 = uextend.i64 v15
;;                                     v38 = iconst.i64 3
;;                                     v39 = ishl v18, v38  ; v38 = 3
;;                                     v37 = iconst.i64 32
;; @0024                               v20 = ushr v39, v37  ; v37 = 32
;; @0024                               trapnz v20, user1
;;                                     v48 = iconst.i32 3
;;                                     v49 = ishl v15, v48  ; v48 = 3
;; @0024                               v22 = iconst.i32 24
;; @0024                               v23 = uadd_overflow_trap v49, v22, user1  ; v22 = 24
;;                                     v56 = ishl v3, v48  ; v48 = 3
;; @0024                               v26 = iadd v56, v22  ; v22 = 24
;; @0024                               v31 = uextend.i64 v26
;; @0024                               v32 = uadd_overflow_trap v8, v31, user1
;; @0024                               v33 = uextend.i64 v23
;; @0024                               v34 = uadd_overflow_trap v8, v33, user1
;; @0024                               v35 = icmp ule v34, v7
;; @0024                               trapz v35, user1
;; @0024                               v36 = iadd v6, v32
;; @0024                               store notrap aligned little v4, v36
;; @0027                               jump block1
;;
;;                                 block1:
;; @0027                               return
;; }
