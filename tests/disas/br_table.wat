;;! target = "x86_64"

(module
  (func (result i32)
    (block (result i32)
      (block (result i32)
        (block (result i32)
          (br_table 0 1 2 3 (i32.const 42) (i32.const 0))
        )
      )
    )
  )
  (func (result i32)
    (block (result i32)
      (block (result i32)
        (block (result i32)
          (br_table 3 2 1 0 (i32.const 42) (i32.const 0))
        )
      )
    )
  )
  (func (result i32)
    (block (result i32)
      (br_table 0 0 1 1 (i32.const 42) (i32.const 0))
    )
  )
  (func (result i32)
    (block (result i32)
      (br_table 1 1 0 0 (i32.const 42) (i32.const 0))
    )
  )
)

;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v6 = iconst.i32 42
;; @0023                               v7 = iconst.i32 0
;; @0025                               br_table v7, block8, [block5, block6, block7]  ; v7 = 0
;;
;;                                 block5:
;; @0025                               jump block4(v6)  ; v6 = 42
;;
;;                                 block6:
;; @0025                               jump block3(v6)  ; v6 = 42
;;
;;                                 block7:
;; @0025                               jump block2(v6)  ; v6 = 42
;;
;;                                 block8:
;; @0025                               jump block1(v6)  ; v6 = 42
;;
;;                                 block4(v5: i32):
;; @002c                               jump block3(v5)
;;
;;                                 block3(v4: i32):
;; @002d                               jump block2(v4)
;;
;;                                 block2(v3: i32):
;; @002e                               jump block1(v3)
;;
;;                                 block1(v2: i32):
;; @002e                               return v2
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0037                               v6 = iconst.i32 42
;; @0039                               v7 = iconst.i32 0
;; @003b                               br_table v7, block8, [block5, block6, block7]  ; v7 = 0
;;
;;                                 block5:
;; @003b                               jump block1(v6)  ; v6 = 42
;;
;;                                 block6:
;; @003b                               jump block2(v6)  ; v6 = 42
;;
;;                                 block7:
;; @003b                               jump block3(v6)  ; v6 = 42
;;
;;                                 block8:
;; @003b                               jump block4(v6)  ; v6 = 42
;;
;;                                 block4(v5: i32):
;; @0042                               jump block3(v5)
;;
;;                                 block3(v4: i32):
;; @0043                               jump block2(v4)
;;
;;                                 block2(v3: i32):
;; @0044                               jump block1(v3)
;;
;;                                 block1(v2: i32):
;; @0044                               return v2
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0049                               v4 = iconst.i32 42
;; @004b                               v5 = iconst.i32 0
;; @004d                               br_table v5, block4, [block3, block3, block4]  ; v5 = 0
;;
;;                                 block3:
;; @004d                               jump block2(v4)  ; v4 = 42
;;
;;                                 block4:
;; @004d                               jump block1(v4)  ; v4 = 42
;;
;;                                 block2(v3: i32):
;; @0054                               jump block1(v3)
;;
;;                                 block1(v2: i32):
;; @0054                               return v2
;; }
;;
;; function u0:3(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0059                               v4 = iconst.i32 42
;; @005b                               v5 = iconst.i32 0
;; @005d                               br_table v5, block4, [block3, block3, block4]  ; v5 = 0
;;
;;                                 block3:
;; @005d                               jump block1(v4)  ; v4 = 42
;;
;;                                 block4:
;; @005d                               jump block2(v4)  ; v4 = 42
;;
;;                                 block2(v3: i32):
;; @0064                               jump block1(v3)
;;
;;                                 block1(v2: i32):
;; @0064                               return v2
;; }
