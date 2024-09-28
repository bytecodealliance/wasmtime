;;! target = "x86_64"
;;! flags = "-W function-references,gc"
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
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0023                               trapz v2, user16
;; @0023                               v9 = uextend.i64 v2
;; @0023                               v10 = iconst.i64 16
;; @0023                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 16
;;                                     v30 = iconst.i64 24
;; @0023                               v13 = uadd_overflow_trap v9, v30, user1  ; v30 = 24
;; @0023                               v8 = load.i64 notrap aligned readonly v0+48
;; @0023                               v14 = icmp ule v13, v8
;; @0023                               trapz v14, user1
;; @0023                               v7 = load.i64 notrap aligned readonly v0+40
;; @0023                               v15 = iadd v7, v11
;; @0023                               v16 = load.f32 notrap aligned little v15
;; @0029                               trapz v2, user16
;; @0029                               v22 = iconst.i64 20
;; @0029                               v23 = uadd_overflow_trap v9, v22, user1  ; v22 = 20
;; @0029                               trapz v14, user1
;; @0029                               v27 = iadd v7, v23
;; @0029                               v28 = load.i8 notrap aligned little v27
;; @002d                               jump block1
;;
;;                                 block1:
;; @0029                               v29 = sextend.i32 v28
;; @002d                               return v16, v29
;; }
