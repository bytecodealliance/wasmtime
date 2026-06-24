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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @0034                               jump block1
;;
;;                                 block1:
;;                                     v15 = iconcat.i64 v2, v3
;;                                     v16 = iconcat.i64 v4, v5
;;                                     v17 = icmp slt v15, v16
;;                                     v23 = uextend.i32 v17
;; @0034                               return v23
;; }
;;
;; function u0:1(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @0047                               jump block1
;;
;;                                 block1:
;;                                     v15 = iconcat.i64 v2, v3
;;                                     v16 = iconcat.i64 v4, v5
;;                                     v17 = icmp ult v15, v16
;;                                     v23 = uextend.i32 v17
;; @0047                               return v23
;; }
;;
;; function u0:2(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @005a                               jump block1
;;
;;                                 block1:
;;                                     v15 = iconcat.i64 v2, v3
;;                                     v16 = iconcat.i64 v4, v5
;;                                     v17 = icmp sle v15, v16
;;                                     v23 = uextend.i32 v17
;; @005a                               return v23
;; }
;;
;; function u0:3(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @006d                               jump block1
;;
;;                                 block1:
;;                                     v15 = iconcat.i64 v2, v3
;;                                     v16 = iconcat.i64 v4, v5
;;                                     v17 = icmp ule v15, v16
;;                                     v23 = uextend.i32 v17
;; @006d                               return v23
;; }
;;
;; function u0:4(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @0080                               jump block1
;;
;;                                 block1:
;;                                     v15 = iconcat.i64 v2, v3
;;                                     v16 = iconcat.i64 v4, v5
;;                                     v17 = icmp sgt v15, v16
;;                                     v23 = uextend.i32 v17
;; @0080                               return v23
;; }
;;
;; function u0:5(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @0093                               jump block1
;;
;;                                 block1:
;;                                     v15 = iconcat.i64 v2, v3
;;                                     v16 = iconcat.i64 v4, v5
;;                                     v17 = icmp ugt v15, v16
;;                                     v23 = uextend.i32 v17
;; @0093                               return v23
;; }
;;
;; function u0:6(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @00a6                               jump block1
;;
;;                                 block1:
;;                                     v15 = iconcat.i64 v2, v3
;;                                     v16 = iconcat.i64 v4, v5
;;                                     v17 = icmp sge v15, v16
;;                                     v23 = uextend.i32 v17
;; @00a6                               return v23
;; }
;;
;; function u0:7(i64 vmctx, i64, i64, i64, i64, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64):
;; @00b9                               jump block1
;;
;;                                 block1:
;;                                     v15 = iconcat.i64 v2, v3
;;                                     v16 = iconcat.i64 v4, v5
;;                                     v17 = icmp uge v15, v16
;;                                     v23 = uextend.i32 v17
;; @00b9                               return v23
;; }
