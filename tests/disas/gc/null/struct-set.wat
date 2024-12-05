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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: f32):
;; @0034                               trapz v2, user16
;; @0034                               v9 = uextend.i64 v2
;; @0034                               v10 = iconst.i64 8
;; @0034                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 8
;;                                     v16 = iconst.i64 24
;; @0034                               v13 = uadd_overflow_trap v9, v16, user1  ; v16 = 24
;; @0034                               v8 = load.i64 notrap aligned readonly v0+48
;; @0034                               v14 = icmp ule v13, v8
;; @0034                               trapz v14, user1
;; @0034                               v6 = load.i64 notrap aligned readonly v0+40
;; @0034                               v15 = iadd v6, v11
;; @0034                               store notrap aligned little v3, v15
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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @003f                               trapz v2, user16
;; @003f                               v9 = uextend.i64 v2
;; @003f                               v10 = iconst.i64 12
;; @003f                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 12
;;                                     v16 = iconst.i64 24
;; @003f                               v13 = uadd_overflow_trap v9, v16, user1  ; v16 = 24
;; @003f                               v8 = load.i64 notrap aligned readonly v0+48
;; @003f                               v14 = icmp ule v13, v8
;; @003f                               trapz v14, user1
;; @003f                               v6 = load.i64 notrap aligned readonly v0+40
;; @003f                               v15 = iadd v6, v11
;; @003f                               istore8 notrap aligned little v3, v15
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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @004a                               trapz v2, user16
;; @004a                               v9 = uextend.i64 v2
;; @004a                               v10 = iconst.i64 16
;; @004a                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 16
;;                                     v16 = iconst.i64 24
;; @004a                               v13 = uadd_overflow_trap v9, v16, user1  ; v16 = 24
;; @004a                               v8 = load.i64 notrap aligned readonly v0+48
;; @004a                               v14 = icmp ule v13, v8
;; @004a                               trapz v14, user1
;; @004a                               v6 = load.i64 notrap aligned readonly v0+40
;; @004a                               v15 = iadd v6, v11
;; @004a                               store notrap aligned little v3, v15
;; @004e                               jump block1
;;
;;                                 block1:
;; @004e                               return
;; }
