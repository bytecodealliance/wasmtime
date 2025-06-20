;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))))

  (func (param (ref null $ty)) (result f32 i32)
    (struct.get $ty 0 (local.get 0))
    (struct.get_s $ty 1 (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> f32, i32 tail {
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
;; @0023                               trapz v2, user16
;; @0023                               v20 = load.i64 notrap aligned readonly can_move v0+8
;; @0023                               v6 = load.i64 notrap aligned readonly can_move v20+24
;; @0023                               v5 = uextend.i64 v2
;; @0023                               v7 = iadd v6, v5
;; @0023                               v8 = iconst.i64 8
;; @0023                               v9 = iadd v7, v8  ; v8 = 8
;; @0023                               v10 = load.f32 notrap aligned little v9
;; @0029                               v14 = iconst.i64 12
;; @0029                               v15 = iadd v7, v14  ; v14 = 12
;; @0029                               v16 = load.i8 notrap aligned little v15
;; @002d                               jump block1
;;
;;                                 block1:
;; @0029                               v17 = sextend.i32 v16
;; @002d                               return v10, v17
;; }
