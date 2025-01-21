;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $t1 (sub (func)))
  (type $t2 (sub $t1 (func)))
  (type $t3 (sub $t2 (func)))

  (import "" "f2" (func $f2 (type $t2)))
  (import "" "f3" (func $f3 (type $t3)))

  (table (ref null $t2) (elem $f2 $f3))

  (func (export "run") (param i32)
    (call_indirect (type $t1) (local.get 0))
  )
)
;; function u0:2(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+136
;;     sig0 = (i64 vmctx, i64) tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     sig2 = (i64 vmctx, i32, i32) -> i32 tail
;;     fn0 = colocated u1:9 sig1
;;     fn1 = colocated u1:35 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005c                               v3 = iconst.i32 2
;; @005c                               v4 = icmp uge v2, v3  ; v3 = 2
;; @005c                               v9 = iconst.i64 0
;; @005c                               v6 = load.i64 notrap aligned readonly v0+136
;; @005c                               v5 = uextend.i64 v2
;;                                     v30 = iconst.i64 3
;; @005c                               v7 = ishl v5, v30  ; v30 = 3
;; @005c                               v8 = iadd v6, v7
;; @005c                               v10 = select_spectre_guard v4, v9, v8  ; v9 = 0
;; @005c                               v11 = load.i64 user5 aligned table v10
;;                                     v31 = iconst.i64 -2
;; @005c                               v12 = band v11, v31  ; v31 = -2
;; @005c                               brif v11, block3(v12), block2
;;
;;                                 block2 cold:
;; @005c                               v14 = iconst.i32 0
;; @005c                               v17 = call fn0(v0, v14, v5)  ; v14 = 0
;; @005c                               jump block3(v17)
;;
;;                                 block3(v13: i64):
;; @005c                               v21 = load.i32 user6 aligned readonly v13+16
;; @005c                               v19 = load.i64 notrap aligned readonly v0+80
;; @005c                               v20 = load.i32 notrap aligned readonly v19
;; @005c                               v22 = icmp eq v21, v20
;; @005c                               v23 = uextend.i32 v22
;; @005c                               brif v23, block5(v23), block4
;;
;;                                 block4:
;; @005c                               v25 = call fn1(v0, v21, v20)
;; @005c                               jump block5(v25)
;;
;;                                 block5(v26: i32):
;; @005c                               trapz v26, user7
;; @005c                               v27 = load.i64 notrap aligned readonly v13+8
;; @005c                               v28 = load.i64 notrap aligned readonly v13+24
;; @005c                               call_indirect sig0, v27(v28, v0)
;; @005f                               jump block1
;;
;;                                 block1:
;; @005f                               return
;; }
