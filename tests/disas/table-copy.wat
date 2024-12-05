;;! target = "x86_64"

(module $n
  (table $t (import "m" "t") 6 funcref)

  (func $i (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 3))
  (func $j (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 4))
  (func $k (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 5))

  (table $u (export "u") funcref (elem $i $j $k $i $j $k))

  (func (export "copy_to_t_from_u") (param i32 i32 i32 i32) (result i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    table.copy $t $u)

  (func (export "copy_to_u_from_t") (param i32 i32 i32 i32) (result i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    table.copy $u $t))

;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32, v7: i32):
;; @007b                               jump block1(v5)
;;
;;                                 block1(v8: i32):
;; @007b                               return v8
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32, i32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32, v7: i32):
;; @0080                               jump block1(v6)
;;
;;                                 block1(v8: i32):
;; @0080                               return v8
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32, i32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32, v7: i32):
;; @0085                               jump block1(v7)
;;
;;                                 block1(v8: i32):
;; @0085                               return v8
;; }
;;
;; function u0:3(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext, i64, i64, i64) tail
;;     fn0 = colocated u1:1 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0090                               v7 = uextend.i64 v3
;; @0090                               v8 = uextend.i64 v4
;; @0090                               v9 = uextend.i64 v5
;; @0090                               v10 = iconst.i32 0
;; @0090                               v11 = iconst.i32 1
;; @0090                               v12 = global_value.i64 gv3
;; @0090                               call fn0(v12, v10, v11, v7, v8, v9)  ; v10 = 0, v11 = 1
;; @0094                               jump block1(v2)
;;
;;                                 block1(v6: i32):
;; @0094                               return v6
;; }
;;
;; function u0:4(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext, i64, i64, i64) tail
;;     fn0 = colocated u1:1 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @009f                               v7 = uextend.i64 v3
;; @009f                               v8 = uextend.i64 v4
;; @009f                               v9 = uextend.i64 v5
;; @009f                               v10 = iconst.i32 1
;; @009f                               v11 = iconst.i32 0
;; @009f                               v12 = global_value.i64 gv3
;; @009f                               call fn0(v12, v10, v11, v7, v8, v9)  ; v10 = 1, v11 = 0
;; @00a3                               jump block1(v2)
;;
;;                                 block1(v6: i32):
;; @00a3                               return v6
;; }
