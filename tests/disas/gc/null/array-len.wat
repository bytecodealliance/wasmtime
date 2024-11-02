;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty)) (result i32)
    (array.len (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001f                               trapz v2, user16
;; @001f                               v8 = uextend.i64 v2
;; @001f                               v9 = iconst.i64 8
;; @001f                               v10 = uadd_overflow_trap v8, v9, user1  ; v9 = 8
;; @001f                               v11 = iconst.i64 4
;; @001f                               v12 = uadd_overflow_trap v10, v11, user1  ; v11 = 4
;; @001f                               v7 = load.i64 notrap aligned readonly v0+48
;; @001f                               v13 = icmp ule v12, v7
;; @001f                               trapz v13, user1
;; @001f                               v5 = load.i64 notrap aligned readonly v0+40
;; @001f                               v14 = iadd v5, v10
;; @001f                               v15 = load.i32 notrap aligned v14
;; @0021                               jump block1
;;
;;                                 block1:
;; @0021                               return v15
;; }
