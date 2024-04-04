;;! target = "x86_64"

(module
  (memory 1)
  (func $main (local i32)
    (i32.store (i32.const 0) (i32.const 0x0))
    (if (i32.load (i32.const 0))
        (then (i32.store (i32.const 0) (i32.const 0xa)))
        (else (i32.store (i32.const 0) (i32.const 0xb))))
  )
  (start $main)
  (data (i32.const 0) "0000")
)

;; function u0:0(i64 vmctx, i64) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @001f                               v2 = iconst.i32 0
;; @0021                               v3 = iconst.i32 0
;; @0023                               v4 = iconst.i32 0
;; @0025                               v5 = uextend.i64 v3  ; v3 = 0
;; @0025                               v6 = global_value.i64 gv4
;; @0025                               v7 = iadd v6, v5
;; @0025                               store little heap v4, v7  ; v4 = 0
;; @0028                               v8 = iconst.i32 0
;; @002a                               v9 = uextend.i64 v8  ; v8 = 0
;; @002a                               v10 = global_value.i64 gv4
;; @002a                               v11 = iadd v10, v9
;; @002a                               v12 = load.i32 little heap v11
;; @002d                               brif v12, block2, block4
;;
;;                                 block2:
;; @002f                               v13 = iconst.i32 0
;; @0031                               v14 = iconst.i32 10
;; @0033                               v15 = uextend.i64 v13  ; v13 = 0
;; @0033                               v16 = global_value.i64 gv4
;; @0033                               v17 = iadd v16, v15
;; @0033                               store little heap v14, v17  ; v14 = 10
;; @0036                               jump block3
;;
;;                                 block4:
;; @0037                               v18 = iconst.i32 0
;; @0039                               v19 = iconst.i32 11
;; @003b                               v20 = uextend.i64 v18  ; v18 = 0
;; @003b                               v21 = global_value.i64 gv4
;; @003b                               v22 = iadd v21, v20
;; @003b                               store little heap v19, v22  ; v19 = 11
;; @003e                               jump block3
;;
;;                                 block3:
;; @003f                               jump block1
;;
;;                                 block1:
;; @003f                               return
;; }
