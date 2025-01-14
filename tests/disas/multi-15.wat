;;! target = "x86_64"

(module
  (func (export "large-sig")
    (param i32 i64 f32 f32 i32 f64 f32 i32 i32 i32 f32 f64 f64 f64 i32 i32 f32)
    (result f64 f32 i32 i32 i32 i64 f32 i32 i32 f32 f64 f64 i32 f32 i32 f64)
    (local.get 5)
    (local.get 2)
    (local.get 0)
    (local.get 8)
    (local.get 7)
    (local.get 1)
    (local.get 3)
    (local.get 9)
    (local.get 4)
    (local.get 6)
    (local.get 13)
    (local.get 11)
    (local.get 15)
    (local.get 16)
    (local.get 14)
    (local.get 12)
  )
)

;; function u0:0(i64 vmctx, i64, i32, i64, f32, f32, i32, f64, f32, i32, i32, i32, f32, f64, f64, f64, i32, i32, f32) -> f64, f32, i32, i32, i32, i64, f32, i32, i32, f32, f64, f64, i32, f32, i32, f64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64, v4: f32, v5: f32, v6: i32, v7: f64, v8: f32, v9: i32, v10: i32, v11: i32, v12: f32, v13: f64, v14: f64, v15: f64, v16: i32, v17: i32, v18: f32):
;; @0067                               jump block1
;;
;;                                 block1:
;; @0067                               return v7, v4, v2, v10, v9, v3, v5, v11, v6, v8, v15, v13, v17, v18, v16, v14
;; }
