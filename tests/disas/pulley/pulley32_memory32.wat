;;! target = "pulley32"
;;! test = "compile"

(module
  (memory 1)

  (func $load8 (param i32) (result i32)
    (i32.load8_u (local.get 0)))

  (func $load16 (param i32) (result i32)
    (i32.load16_u (local.get 0)))

  (func $load32 (param i32) (result i32)
    (i32.load (local.get 0)))

  (func $load64 (param i32) (result i64)
    (i64.load (local.get 0)))

  (func $load8_offset (param i32) (result i32)
    (i32.load8_u offset=32 (local.get 0)))

  (func $load16_offset (param i32) (result i32)
    (i32.load16_u offset=32 (local.get 0)))

  (func $load32_offset (param i32) (result i32)
    (i32.load offset=32 (local.get 0)))

  (func $load64_offset (param i32) (result i64)
    (i64.load offset=32 (local.get 0)))
)
;; wasm[0]::function[0]::load8:
;;       push_frame
;;       xload32le_offset8 x6, x0, 52
;;       xbc32_bound32_trap x2, x6, 1
;;       xload32le_offset8 x7, x0, 48
;;       xadd32 x7, x7, x2
;;       xload8_u32_offset8 x0, x7, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[1]::load16:
;;       push_frame
;;       xload32le_offset8 x6, x0, 52
;;       xbc32_bound32_trap x2, x6, 2
;;       xload32le_offset8 x7, x0, 48
;;       xadd32 x7, x7, x2
;;       xload16le_u32_offset8 x0, x7, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[2]::load32:
;;       push_frame
;;       xload32le_offset8 x6, x0, 52
;;       xbc32_bound32_trap x2, x6, 4
;;       xload32le_offset8 x7, x0, 48
;;       xadd32 x7, x7, x2
;;       xload32le_offset8 x0, x7, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[3]::load64:
;;       push_frame
;;       xload32le_offset8 x6, x0, 52
;;       xbc32_bound32_trap x2, x6, 8
;;       xload32le_offset8 x7, x0, 48
;;       xadd32 x7, x7, x2
;;       xload64le_offset8 x0, x7, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[4]::load8_offset:
;;       push_frame
;;       xload32le_offset8 x6, x0, 52
;;       xbc32_bound32_trap x2, x6, 33
;;       xload32le_offset8 x7, x0, 48
;;       xadd32 x7, x7, x2
;;       xload8_u32_offset8 x0, x7, 32
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[5]::load16_offset:
;;       push_frame
;;       xload32le_offset8 x6, x0, 52
;;       xbc32_bound32_trap x2, x6, 34
;;       xload32le_offset8 x7, x0, 48
;;       xadd32 x7, x7, x2
;;       xload16le_u32_offset8 x0, x7, 32
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[6]::load32_offset:
;;       push_frame
;;       xload32le_offset8 x6, x0, 52
;;       xbc32_bound32_trap x2, x6, 36
;;       xload32le_offset8 x7, x0, 48
;;       xadd32 x7, x7, x2
;;       xload32le_offset8 x0, x7, 32
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[7]::load64_offset:
;;       push_frame
;;       xload32le_offset8 x6, x0, 52
;;       xbc32_bound32_trap x2, x6, 40
;;       xload32le_offset8 x7, x0, 48
;;       xadd32 x7, x7, x2
;;       xload64le_offset8 x0, x7, 32
;;       pop_frame
;;       ret
