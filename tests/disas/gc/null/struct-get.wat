;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (func (param (ref null $ty)) (result f32)
    (struct.get $ty 0 (local.get 0))
  )

  (func (param (ref null $ty)) (result i32)
    (struct.get_s $ty 1 (local.get 0))
  )

  (func (param (ref null $ty)) (result i32)
    (struct.get_u $ty 1 (local.get 0))
  )

  (func (param (ref null $ty)) (result anyref)
    (struct.get $ty 2 (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> f32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0033                               trapz v2, user16
;; @0033                               v9 = uextend.i64 v2
;; @0033                               v10 = iconst.i64 8
;; @0033                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 8
;;                                     v17 = iconst.i64 24
;; @0033                               v13 = uadd_overflow_trap v9, v17, user1  ; v17 = 24
;; @0033                               v8 = load.i64 notrap aligned readonly v0+48
;; @0033                               v14 = icmp ule v13, v8
;; @0033                               trapz v14, user1
;; @0033                               v6 = load.i64 notrap aligned readonly v0+40
;; @0033                               v15 = iadd v6, v11
;; @0033                               v16 = load.f32 notrap aligned little v15
;; @0037                               jump block1
;;
;;                                 block1:
;; @0037                               return v16
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003c                               trapz v2, user16
;; @003c                               v9 = uextend.i64 v2
;; @003c                               v10 = iconst.i64 12
;; @003c                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 12
;;                                     v18 = iconst.i64 24
;; @003c                               v13 = uadd_overflow_trap v9, v18, user1  ; v18 = 24
;; @003c                               v8 = load.i64 notrap aligned readonly v0+48
;; @003c                               v14 = icmp ule v13, v8
;; @003c                               trapz v14, user1
;; @003c                               v6 = load.i64 notrap aligned readonly v0+40
;; @003c                               v15 = iadd v6, v11
;; @003c                               v16 = load.i8 notrap aligned little v15
;; @0040                               jump block1
;;
;;                                 block1:
;; @003c                               v17 = sextend.i32 v16
;; @0040                               return v17
;; }
;;
;; function u0:2(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0045                               trapz v2, user16
;; @0045                               v9 = uextend.i64 v2
;; @0045                               v10 = iconst.i64 12
;; @0045                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 12
;;                                     v18 = iconst.i64 24
;; @0045                               v13 = uadd_overflow_trap v9, v18, user1  ; v18 = 24
;; @0045                               v8 = load.i64 notrap aligned readonly v0+48
;; @0045                               v14 = icmp ule v13, v8
;; @0045                               trapz v14, user1
;; @0045                               v6 = load.i64 notrap aligned readonly v0+40
;; @0045                               v15 = iadd v6, v11
;; @0045                               v16 = load.i8 notrap aligned little v15
;; @0049                               jump block1
;;
;;                                 block1:
;; @0045                               v17 = uextend.i32 v16
;; @0049                               return v17
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @004e                               trapz v2, user16
;; @004e                               v9 = uextend.i64 v2
;; @004e                               v10 = iconst.i64 16
;; @004e                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 16
;;                                     v17 = iconst.i64 24
;; @004e                               v13 = uadd_overflow_trap v9, v17, user1  ; v17 = 24
;; @004e                               v8 = load.i64 notrap aligned readonly v0+48
;; @004e                               v14 = icmp ule v13, v8
;; @004e                               trapz v14, user1
;; @004e                               v6 = load.i64 notrap aligned readonly v0+40
;; @004e                               v15 = iadd v6, v11
;; @004e                               v16 = load.i32 notrap aligned little v15
;; @0052                               jump block1
;;
;;                                 block1:
;; @0052                               return v16
;; }
