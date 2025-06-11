;;! target = "pulley64"
;;! test = "compile"

(module
  (memory $m1 1 2)

  (func $offset0 (result i32) (i32.const 0) i32.load $m1)
  (func $offset100 (result i32) (i32.const 100) i32.load $m1)
  (func $offset_mixed (result i32) (i32.const 100) i32.load $m1 offset=100)
  (func $offset_just_ok (result i32) (i32.const 65532) i32.load $m1)
  (func $offset_just_bad (result i32) (i32.const 65533) i32.load $m1)
  (func $offset_just_ok_v2 (result i32) (i32.const 1) i32.load $m1 offset=65531)
  (func $offset_just_bad_v2 (result i32) (i32.const 1) i32.load $m1 offset=65532)

  (func $maybe_inbounds (result i32) (i32.const 131068) i32.load $m1)
  (func $maybe_inbounds_v2 (result i32) (i32.const 0) i32.load $m1 offset=131068)
  (func $never_inbounds (result i32) (i32.const 131069) i32.load $m1)
  (func $never_inbounds_v2 (result i32) (i32.const 0) i32.load $m1 offset=131069)
)

;; wasm[0]::function[0]::offset0:
;;       push_frame
;;       xload64le_o32 x3, x0, 56
;;       xload32le_z x0, x3, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[1]::offset100:
;;       push_frame
;;       xload64le_o32 x3, x0, 56
;;       xload32le_z x0, x3, 100
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[2]::offset_mixed:
;;       push_frame
;;       xload64le_o32 x3, x0, 56
;;       xload32le_z x0, x3, 200
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[3]::offset_just_ok:
;;       push_frame
;;       xload64le_o32 x3, x0, 56
;;       xload32le_z x0, x3, 65532
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[4]::offset_just_bad:
;;       push_frame
;;       xload64le_o32 x5, x0, 64
;;       xconst32 x6, 65533
;;       xload64le_o32 x7, x0, 56
;;       xload32le_g32 x0, x7, x5, x6, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[5]::offset_just_ok_v2:
;;       push_frame
;;       xload64le_o32 x3, x0, 56
;;       xload32le_z x0, x3, 65532
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[6]::offset_just_bad_v2:
;;       push_frame
;;       xload64le_o32 x9, x0, 64
;;       xzero x10
;;       xload64le_o32 x11, x0, 56
;;       xadd64_u32 x11, x11, 65533
;;       xconst32 x7, 65536
;;       xeq64 x9, x9, x7
;;       xselect64 x11, x9, x10, x11
;;       xload32le_z x0, x11, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[7]::maybe_inbounds:
;;       push_frame
;;       xload64le_o32 x5, x0, 64
;;       xconst32 x6, 131068
;;       xload64le_o32 x7, x0, 56
;;       xload32le_g32 x0, x7, x5, x6, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[8]::maybe_inbounds_v2:
;;       push_frame
;;       xload64le_o32 x9, x0, 64
;;       xzero x10
;;       xload64le_o32 x11, x0, 56
;;       xadd64_u32 x11, x11, 131068
;;       xconst32 x7, 131072
;;       xult64 x9, x9, x7
;;       xselect64 x11, x9, x10, x11
;;       xload32le_z x0, x11, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[9]::never_inbounds:
;;       push_frame
;;       xload64le_o32 x5, x0, 64
;;       xconst32 x6, 131069
;;       xload64le_o32 x7, x0, 56
;;       xload32le_g32 x0, x7, x5, x6, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[10]::never_inbounds_v2:
;;       push_frame
;;       trap
