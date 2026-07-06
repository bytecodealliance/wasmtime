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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u0:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0025                               v5 = iconst.i32 0
;; @0027                               v6 = iconst.i32 0
;; @0029                               v7 = iconst.i32 0
;; @002b                               v8 = iconst.i32 0
;; @002d                               v9 = iconst.i32 0
;; @002f                               brif v9, block2, block4  ; v9 = 0
;;
;;                                 block2:
;; @0031                               jump block3(v8)  ; v8 = 0
;;
;;                                 block4:
;; @0034                               v10 = call fn0(v0, v0, v6, v7, v8)  ; v6 = 0, v7 = 0, v8 = 0
;; @0036                               jump block3(v10)
;;
;;                                 block3(v11: i32):
;; @0037                               v12 = iconst.i32 0
;; @0039                               v13 = iconst.i32 0
;; @003b                               brif v13, block5, block7  ; v13 = 0
;;
;;                                 block5:
;; @003f                               jump block6
;;
;;                                 block7:
;; @0042                               jump block6
;;
;;                                 block6:
;; @0043                               jump block1
;;
;;                                 block1:
;; @0043                               return v5  ; v5 = 0
;; }
