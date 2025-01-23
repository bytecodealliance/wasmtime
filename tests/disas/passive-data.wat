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

;; function u0:0(i64 vmctx, i64, i32, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i64 notrap aligned readonly checked gv3+80
;;     sig0 = (i64 vmctx, i32, i32, i64, i32, i32) -> i8 tail
;;     fn0 = colocated u1:6 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @003d                               v5 = iconst.i32 0
;; @003d                               v6 = iconst.i32 0
;; @003d                               v8 = uextend.i64 v2
;; @003d                               v9 = call fn0(v0, v5, v6, v8, v3, v4)  ; v5 = 0, v6 = 0
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return
;; }
;;
;; function u0:1(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:8 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0044                               v2 = iconst.i32 0
;; @0044                               call fn0(v0, v2)  ; v2 = 0
;; @0047                               jump block1
;;
;;                                 block1:
;; @0047                               return
;; }
