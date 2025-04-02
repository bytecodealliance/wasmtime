;;! target = "pulley32"
;;! test = "compile"

(module
  (func $madd32 (param i32 i32 i32) (result i32)
    (i32.add
      (i32.mul (local.get 0) (local.get 1))
      (local.get 2)))

  (func $madd64 (param i64 i64 i64) (result i64)
    (i64.add
      (i64.mul (local.get 0) (local.get 1))
      (local.get 2)))
)
;; wasm[0]::function[0]::madd32:
;;       push_frame
;;       xmadd32 x0, x2, x3, x4
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[1]::madd64:
;;       push_frame
;;       xmadd64 x0, x2, x3, x4
;;       pop_frame
;;       ret
