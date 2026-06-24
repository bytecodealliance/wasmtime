;;! target = "x86_64"

(module
  (func (export "multiIf2") (param i32 i64 i64) (result i64 i64)
    (local.get 2)
    (local.get 1)
    (local.get 0)
    (if (param i64 i64) (result i64 i64)
      (then
        i64.add
        i64.const 1)
      ;; Hits the code path for an `else` after a block that does not end unreachable.
      (else
        i64.sub
        i64.const 2))))

;; function u0:0(i64 vmctx, i64, i32, i64, i64) -> i64, i64 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64, v4: i64):
;; @0037                               brif v2, block2, block4
;;
;;                                 block2:
;; @0039                               v5 = iadd.i64 v4, v3
;; @003a                               v6 = iconst.i64 1
;; @003c                               jump block3(v5, v6)  ; v6 = 1
;;
;;                                 block4:
;; @003d                               v7 = isub.i64 v4, v3
;; @003e                               v8 = iconst.i64 2
;; @0040                               jump block3(v7, v8)  ; v8 = 2
;;
;;                                 block3(v9: i64, v10: i64):
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return v9, v10
;; }
