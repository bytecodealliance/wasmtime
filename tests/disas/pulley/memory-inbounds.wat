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
;;       xload64le_o32 x3, x0, 80
;;       xload32le_z x0, x3, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[1]::offset100:
;;       push_frame
;;       xload64le_o32 x4, x0, 80
;;       xadd64_u8 x4, x4, 100
;;       xload32le_z x0, x4, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[2]::offset_mixed:
;;       push_frame
;;       xload64le_o32 x4, x0, 80
;;       xadd64_u8 x4, x4, 200
;;       xload32le_z x0, x4, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[3]::offset_just_ok:
;;       push_frame
;;       xload64le_o32 x4, x0, 80
;;       xadd64_u32 x4, x4, 65532
;;       xload32le_z x0, x4, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[4]::offset_just_bad:
;;       push_frame
;;       xload64le_o32 x10, x0, 88
;;       xsub64_u8 x10, x10, 4
;;       xzero x11
;;       xload64le_o32 x12, x0, 80
;;       xadd64_u32 x12, x12, 65533
;;       xconst32 x8, 65533
;;       xult64 x10, x10, x8
;;       xselect64 x12, x10, x11, x12
;;       xload32le_z x0, x12, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[5]::offset_just_ok_v2:
;;       push_frame
;;       xload64le_o32 x4, x0, 80
;;       xadd64_u32 x4, x4, 65532
;;       xload32le_z x0, x4, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[6]::offset_just_bad_v2:
;;       push_frame
;;       xload64le_o32 x10, x0, 88
;;       xsub64_u32 x10, x10, 65536
;;       xzero x11
;;       xload64le_o32 x12, x0, 80
;;       xadd64_u32 x12, x12, 65533
;;       xzero x8
;;       xeq64 x10, x10, x8
;;       xselect64 x12, x10, x11, x12
;;       xload32le_z x0, x12, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[7]::maybe_inbounds:
;;       push_frame
;;       xload64le_o32 x10, x0, 88
;;       xsub64_u8 x10, x10, 4
;;       xzero x11
;;       xload64le_o32 x12, x0, 80
;;       xadd64_u32 x12, x12, 131068
;;       xconst32 x8, 131068
;;       xult64 x10, x10, x8
;;       xselect64 x12, x10, x11, x12
;;       xload32le_z x0, x12, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[8]::maybe_inbounds_v2:
;;       push_frame
;;       xzero x10
;;       xconst32 x11, 131072
;;       xadd64_uoverflow_trap x11, x10, x11
;;       xload64le_o32 x12, x0, 88
;;       xload64le_o32 x13, x0, 80
;;       xadd64_u32 x13, x13, 131068
;;       xult64 x9, x12, x11
;;       xselect64 x11, x9, x10, x13
;;       xload32le_z x0, x11, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[9]::never_inbounds:
;;       push_frame
;;       xload64le_o32 x10, x0, 88
;;       xsub64_u8 x10, x10, 4
;;       xzero x11
;;       xload64le_o32 x12, x0, 80
;;       xadd64_u32 x12, x12, 131069
;;       xconst32 x8, 131069
;;       xult64 x10, x10, x8
;;       xselect64 x12, x10, x11, x12
;;       xload32le_z x0, x12, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[10]::never_inbounds_v2:
;;       push_frame
;;       trap
