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

  (func $store8 (param i32 i32)
    (i32.store8 (local.get 0) (local.get 1)))

  (func $store16 (param i32 i32)
    (i32.store16 (local.get 0) (local.get 1)))

  (func $store32 (param i32 i32)
    (i32.store (local.get 0) (local.get 1)))

  (func $store64 (param i32 i64)
    (i64.store (local.get 0) (local.get 1)))

  (func $load8_offset (param i32) (result i32)
    (i32.load8_u offset=32 (local.get 0)))

  (func $load16_offset (param i32) (result i32)
    (i32.load16_u offset=32 (local.get 0)))

  (func $load32_offset (param i32) (result i32)
    (i32.load offset=32 (local.get 0)))

  (func $load64_offset (param i32) (result i64)
    (i64.load offset=32 (local.get 0)))

  (func $store8_offset (param i32 i32)
    (i32.store8 offset=8 (local.get 0) (local.get 1)))

  (func $store16_offset (param i32 i32)
    (i32.store16 offset=8 (local.get 0) (local.get 1)))

  (func $store32_offset (param i32 i32)
    (i32.store offset=8 (local.get 0) (local.get 1)))

  (func $store64_offset (param i32 i64)
    (i64.store offset=8 (local.get 0) (local.get 1)))
)
;; wasm[0]::function[0]::load8:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xload8_u32_g32 x0, x6, x5, x2, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[1]::load16:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xload16le_u32_g32 x0, x6, x5, x2, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[2]::load32:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xload32le_g32 x0, x6, x5, x2, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[3]::load64:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xload64le_g32 x0, x6, x5, x2, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[4]::store8:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xstore8_g32 x6, x5, x2, 0, x3
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[5]::store16:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xstore16le_g32 x6, x5, x2, 0, x3
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[6]::store32:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xstore32le_g32 x6, x5, x2, 0, x3
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[7]::store64:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xstore64le_g32 x6, x5, x2, 0, x3
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[8]::load8_offset:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xload8_u32_g32 x0, x6, x5, x2, 32
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[9]::load16_offset:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xload16le_u32_g32 x0, x6, x5, x2, 32
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[10]::load32_offset:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xload32le_g32 x0, x6, x5, x2, 32
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[11]::load64_offset:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xload64le_g32 x0, x6, x5, x2, 32
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[12]::store8_offset:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xstore8_g32 x6, x5, x2, 8, x3
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[13]::store16_offset:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xstore16le_g32 x6, x5, x2, 8, x3
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[14]::store32_offset:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xstore32le_g32 x6, x5, x2, 8, x3
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[15]::store64_offset:
;;       push_frame
;;       xload32le_o32 x5, x0, 44
;;       xload32le_o32 x6, x0, 40
;;       xstore64le_g32 x6, x5, x2, 8, x3
;;       pop_frame
;;       ret
