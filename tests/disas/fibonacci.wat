;;! target = "x86_64"

(module
  (memory 1)
  (func $main (local i32 i32 i32 i32)
    (local.set 0 (i32.const 0))
    (local.set 1 (i32.const 1))
    (local.set 2 (i32.const 1))
    (local.set 3 (i32.const 0))
    (block
    (loop
        (br_if 1 (i32.gt_s (local.get 0) (i32.const 5)))
        (local.set 3 (local.get 2))
        (local.set 2 (i32.add (local.get 2) (local.get 1)))
        (local.set 1 (local.get 3))
        (local.set 0 (i32.add (local.get 0) (i32.const 1)))
        (br 0)
    )
    )
    (i32.store (i32.const 0) (local.get 2))
  )
  (start $main)
  (data (i32.const 0) "0000")
)

;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i64 notrap aligned readonly checked gv3+80
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @001f                               v2 = iconst.i32 0
;; @0021                               v3 = iconst.i32 0
;; @0025                               v4 = iconst.i32 1
;; @0029                               v5 = iconst.i32 1
;; @002d                               v6 = iconst.i32 0
;; @0033                               jump block3(v3, v5, v4)  ; v3 = 0, v5 = 1, v4 = 1
;;
;;                                 block3(v7: i32, v11: i32, v12: i32):
;; @0037                               v8 = iconst.i32 5
;; @0039                               v9 = icmp sgt v7, v8  ; v8 = 5
;; @0039                               v10 = uextend.i32 v9
;; @003a                               brif v10, block2, block5
;;
;;                                 block5:
;; @0044                               v13 = iadd.i32 v11, v12
;; @004d                               v14 = iconst.i32 1
;; @004f                               v15 = iadd.i32 v7, v14  ; v14 = 1
;; @0052                               jump block3(v15, v13, v11)
;;
;;                                 block2:
;; @0056                               v16 = iconst.i32 0
;; @005a                               v17 = uextend.i64 v16  ; v16 = 0
;; @005a                               v18 = load.i64 notrap aligned readonly checked v0+80
;; @005a                               v19 = iadd v18, v17
;; @005a                               store.i32 little heap v11, v19
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return
;; }
