;;! target = "x86_64"

(module
  (data $passive "this is a passive data segment")
  (memory 0)

  (func (export "init") (param i32 i32 i32)
    local.get 0 ;; dst
    local.get 1 ;; src
    local.get 2 ;; cnt
    memory.init $passive)

  (func (export "drop")
    data.drop $passive))

;; function u0:0(i64 vmctx, i64, i32, i32, i32) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly checked gv3+80
;;     sig0 = (i64 vmctx, i32 uext, i32 uext, i64, i32 uext, i32 uext) system_v
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig2 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @003d                               v5 = iconst.i32 0
;; @003d                               v6 = iconst.i32 0
;; @003d                               v7 = global_value.i64 gv3
;; @003d                               v8 = load.i64 notrap aligned readonly v7+56
;; @003d                               v9 = load.i64 notrap aligned readonly v8+48
;; @003d                               v10 = uextend.i64 v2
;; @003d                               call_indirect sig0, v9(v7, v5, v6, v10, v3, v4)  ; v5 = 0, v6 = 0
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return
;; }
;;
;; function u0:1(i64 vmctx, i64) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext) system_v
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig2 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0044                               v2 = iconst.i32 0
;; @0044                               v3 = global_value.i64 gv3
;; @0044                               v4 = load.i64 notrap aligned readonly v3+56
;; @0044                               v5 = load.i64 notrap aligned readonly v4+64
;; @0044                               call_indirect sig0, v5(v3, v2)  ; v2 = 0
;; @0047                               jump block1
;;
;;                                 block1:
;; @0047                               return
;; }
