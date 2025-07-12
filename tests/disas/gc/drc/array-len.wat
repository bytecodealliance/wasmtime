;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001f                               trapz v2, user16
;; @001f                               v10 = load.i64 notrap aligned readonly can_move v0+8
;; @001f                               v5 = load.i64 notrap aligned readonly can_move v10+24
;; @001f                               v4 = uextend.i64 v2
;; @001f                               v6 = iadd v5, v4
;; @001f                               v7 = iconst.i64 24
;; @001f                               v8 = iadd v6, v7  ; v7 = 24
;; @001f                               v9 = load.i32 notrap aligned readonly v8
;; @0021                               jump block1
;;
;;                                 block1:
;; @0021                               return v9
;; }
