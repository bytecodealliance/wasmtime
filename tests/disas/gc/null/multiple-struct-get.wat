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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0023                               trapz v2, user16
;; @0023                               v10 = uextend.i64 v2
;; @0023                               v11 = iconst.i64 8
;; @0023                               v12 = uadd_overflow_trap v10, v11, user1  ; v11 = 8
;;                                     v32 = iconst.i64 16
;; @0023                               v14 = uadd_overflow_trap v10, v32, user1  ; v32 = 16
;; @0023                               v9 = load.i64 notrap aligned readonly v0+48
;; @0023                               v15 = icmp ule v14, v9
;; @0023                               trapz v15, user1
;; @0023                               v7 = load.i64 notrap aligned readonly v0+40
;; @0023                               v16 = iadd v7, v12
;; @0023                               v17 = load.f32 notrap aligned little v16
;; @0029                               v24 = iconst.i64 12
;; @0029                               v25 = uadd_overflow_trap v10, v24, user1  ; v24 = 12
;; @0029                               v29 = iadd v7, v25
;; @0029                               v30 = load.i8 notrap aligned little v29
;; @002d                               jump block1
;;
;;                                 block1:
;; @0029                               v31 = sextend.i32 v30
;; @002d                               return v17, v31
;; }
