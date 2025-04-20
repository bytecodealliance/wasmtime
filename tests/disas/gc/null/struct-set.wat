;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (func (param (ref null $ty) f32)
    (struct.set $ty 0 (local.get 0) (local.get 1))
  )

  (func (param (ref null $ty) i32)
    (struct.set $ty 1 (local.get 0) (local.get 1))
  )

  (func (param (ref null $ty) anyref)
    (struct.set $ty 2 (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i32, f32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: f32):
;; @0034                               trapz v2, user16
;; @0034                               v9 = load.i64 notrap aligned readonly can_move v0+8
;; @0034                               v5 = load.i64 notrap aligned readonly can_move v9+24
;; @0034                               v4 = uextend.i64 v2
;; @0034                               v6 = iadd v5, v4
;; @0034                               v7 = iconst.i64 8
;; @0034                               v8 = iadd v6, v7  ; v7 = 8
;; @0034                               store notrap aligned little v3, v8
;; @0038                               jump block1
;;
;;                                 block1:
;; @0038                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @003f                               trapz v2, user16
;; @003f                               v9 = load.i64 notrap aligned readonly can_move v0+8
;; @003f                               v5 = load.i64 notrap aligned readonly can_move v9+24
;; @003f                               v4 = uextend.i64 v2
;; @003f                               v6 = iadd v5, v4
;; @003f                               v7 = iconst.i64 12
;; @003f                               v8 = iadd v6, v7  ; v7 = 12
;; @003f                               istore8 notrap aligned little v3, v8
;; @0043                               jump block1
;;
;;                                 block1:
;; @0043                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @004a                               trapz v2, user16
;; @004a                               v9 = load.i64 notrap aligned readonly can_move v0+8
;; @004a                               v5 = load.i64 notrap aligned readonly can_move v9+24
;; @004a                               v4 = uextend.i64 v2
;; @004a                               v6 = iadd v5, v4
;; @004a                               v7 = iconst.i64 16
;; @004a                               v8 = iadd v6, v7  ; v7 = 16
;; @004a                               store notrap aligned little v3, v8
;; @004e                               jump block1
;;
;;                                 block1:
;; @004e                               return
;; }
