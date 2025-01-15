;;! target = "x86_64"

(module
  (func $main (type 0) (param i32 i32 i32) (result i32)
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0

    i32.const 0
    if (param i32 i32 i32) (result i32)  ;; label = @1
      br 0 (;@1;)
    else
      call $main
    end

    i32.const 0

    i32.const 0
    if (param i32 i32 i32) (result i32)  ;; label = @1
      drop
      drop
    else
      drop
      drop
    end
  )
  (export "main" (func $main)))

;; function u0:0(i64 vmctx, i64, i32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     sig0 = (i64 vmctx, i64, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u0:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0025                               v6 = iconst.i32 0
;; @0027                               v7 = iconst.i32 0
;; @0029                               v8 = iconst.i32 0
;; @002b                               v9 = iconst.i32 0
;; @002d                               v10 = iconst.i32 0
;; @002f                               brif v10, block2, block4  ; v10 = 0
;;
;;                                 block2:
;; @0031                               jump block3(v9)  ; v9 = 0
;;
;;                                 block4:
;; @0034                               v15 = call fn0(v0, v0, v7, v8, v9)  ; v7 = 0, v8 = 0, v9 = 0
;; @0036                               jump block3(v15)
;;
;;                                 block3(v11: i32):
;; @0037                               v16 = iconst.i32 0
;; @0039                               v17 = iconst.i32 0
;; @003b                               brif v17, block5, block7(v11)  ; v17 = 0
;;
;;                                 block5:
;; @003f                               jump block6
;;
;;                                 block7(v20: i32):
;; @0042                               jump block6
;;
;;                                 block6:
;; @0043                               jump block1
;;
;;                                 block1:
;; @0043                               return v6  ; v6 = 0
;; }
