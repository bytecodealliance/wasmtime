;;! target = "x86_64"
;;! test = "optimize"

(module
  (func $lt_s (param i64 i64 i64 i64) (result i32)
    local.get 0
    local.get 2
    i64.lt_u
    local.get 1
    local.get 3
    i64.lt_s
    local.get 1
    local.get 3
    i64.eq
    select
  )
  (func $lt_u (param i64 i64 i64 i64) (result i32)
    local.get 0
    local.get 2
    i64.lt_u
    local.get 1
    local.get 3
    i64.lt_u
    local.get 1
    local.get 3
    i64.eq
    select
  )
  (func $le_s (param i64 i64 i64 i64) (result i32)
    local.get 0
    local.get 2
    i64.le_u
    local.get 1
    local.get 3
    i64.le_s
    local.get 1
    local.get 3
    i64.eq
    select
  )
  (func $le_u (param i64 i64 i64 i64) (result i32)
    local.get 0
    local.get 2
    i64.le_u
    local.get 1
    local.get 3
    i64.le_u
    local.get 1
    local.get 3
    i64.eq
    select
  )
  (func $gt_s (param i64 i64 i64 i64) (result i32)
    local.get 0
    local.get 2
    i64.gt_u
    local.get 1
    local.get 3
    i64.gt_s
    local.get 1
    local.get 3
    i64.eq
    select
  )
  (func $gt_u (param i64 i64 i64 i64) (result i32)
    local.get 0
    local.get 2
    i64.gt_u
    local.get 1
    local.get 3
    i64.gt_u
    local.get 1
    local.get 3
    i64.eq
    select
  )
  (func $ge_s (param i64 i64 i64 i64) (result i32)
    local.get 0
    local.get 2
    i64.ge_u
    local.get 1
    local.get 3
    i64.ge_s
    local.get 1
    local.get 3
    i64.eq
    select
  )
  (func $ge_u (param i64 i64 i64 i64) (result i32)
    local.get 0
    local.get 2
    i64.ge_u
    local.get 1
    local.get 3
    i64.ge_u
    local.get 1
    local.get 3
    i64.eq
    select
  )
)
;; function u0:0(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @0034                               jump block1
;;
;;                                 block1:
;;                                     v16 = iconcat.i64 v2, v3
;;                                     v17 = iconcat.i64 v4, v5
;;                                     v18 = icmp slt v16, v17
;;                                     v20 = uextend.i32 v18
;; @0034                               return v20
;; }
;;
;; function u0:1(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @0047                               jump block1
;;
;;                                 block1:
;;                                     v16 = iconcat.i64 v2, v3
;;                                     v17 = iconcat.i64 v4, v5
;;                                     v18 = icmp ult v16, v17
;;                                     v20 = uextend.i32 v18
;; @0047                               return v20
;; }
;;
;; function u0:2(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @005a                               jump block1
;;
;;                                 block1:
;;                                     v16 = iconcat.i64 v2, v3
;;                                     v17 = iconcat.i64 v4, v5
;;                                     v18 = icmp sle v16, v17
;;                                     v20 = uextend.i32 v18
;; @005a                               return v20
;; }
;;
;; function u0:3(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @006d                               jump block1
;;
;;                                 block1:
;;                                     v16 = iconcat.i64 v2, v3
;;                                     v17 = iconcat.i64 v4, v5
;;                                     v18 = icmp ule v16, v17
;;                                     v20 = uextend.i32 v18
;; @006d                               return v20
;; }
;;
;; function u0:4(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @0080                               jump block1
;;
;;                                 block1:
;;                                     v16 = iconcat.i64 v2, v3
;;                                     v17 = iconcat.i64 v4, v5
;;                                     v18 = icmp sgt v16, v17
;;                                     v20 = uextend.i32 v18
;; @0080                               return v20
;; }
;;
;; function u0:5(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @0093                               jump block1
;;
;;                                 block1:
;;                                     v16 = iconcat.i64 v2, v3
;;                                     v17 = iconcat.i64 v4, v5
;;                                     v18 = icmp ugt v16, v17
;;                                     v20 = uextend.i32 v18
;; @0093                               return v20
;; }
;;
;; function u0:6(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @00a6                               jump block1
;;
;;                                 block1:
;;                                     v16 = iconcat.i64 v2, v3
;;                                     v17 = iconcat.i64 v4, v5
;;                                     v18 = icmp sge v16, v17
;;                                     v20 = uextend.i32 v18
;; @00a6                               return v20
;; }
;;
;; function u0:7(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @00b9                               jump block1
;;
;;                                 block1:
;;                                     v16 = iconcat.i64 v2, v3
;;                                     v17 = iconcat.i64 v4, v5
;;                                     v18 = icmp uge v16, v17
;;                                     v20 = uextend.i32 v18
;; @00b9                               return v20
;; }
