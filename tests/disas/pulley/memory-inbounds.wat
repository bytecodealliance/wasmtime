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
;;       xload64le_offset32 x3, x0, 96
;;       xload32le_offset32 x0, x3, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[1]::offset100:
;;       push_frame
;;       xload64le_offset32 x3, x0, 96
;;       xload32le_offset32 x0, x3, 100
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[2]::offset_mixed:
;;       push_frame
;;       xload64le_offset32 x3, x0, 96
;;       xload32le_offset32 x0, x3, 200
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[3]::offset_just_ok:
;;       push_frame
;;       xload64le_offset32 x3, x0, 96
;;       xload32le_offset32 x0, x3, 65532
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[4]::offset_just_bad:
;;       push_frame
;;       xload64le_offset32 x7, x0, 104
;;       xconst8 x8, 4
;;       xsub64 x7, x7, x8
;;       xconst32 x8, 65533
;;       br_if_xult64 x7, x8, 0x17    // target = 0x2b
;;   1b: xload64le_offset32 x8, x0, 96
;;       xload32le_offset32 x0, x8, 65533
;;       pop_frame
;;       ret
;;   2b: trap
;;
;; wasm[0]::function[5]::offset_just_ok_v2:
;;       push_frame
;;       xload64le_offset32 x3, x0, 96
;;       xload32le_offset32 x0, x3, 65532
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[6]::offset_just_bad_v2:
;;       push_frame
;;       xload64le_offset32 x7, x0, 104
;;       xconst32 x8, 65536
;;       xsub64 x7, x7, x8
;;       xconst8 x8, 0
;;       br_if_xeq64 x7, x8, 0x17    // target = 0x2b
;;   1b: xload64le_offset32 x8, x0, 96
;;       xload32le_offset32 x0, x8, 65533
;;       pop_frame
;;       ret
;;   2b: trap
;;
;; wasm[0]::function[7]::maybe_inbounds:
;;       push_frame
;;       xload64le_offset32 x7, x0, 104
;;       xconst8 x8, 4
;;       xsub64 x7, x7, x8
;;       xconst32 x8, 131068
;;       br_if_xult64 x7, x8, 0x17    // target = 0x2b
;;   1b: xload64le_offset32 x8, x0, 96
;;       xload32le_offset32 x0, x8, 131068
;;       pop_frame
;;       ret
;;   2b: trap
;;
;; wasm[0]::function[8]::maybe_inbounds_v2:
;;       push_frame
;;       xconst8 x7, 0
;;       xconst32 x8, 131072
;;       xadd64_uoverflow_trap x7, x7, x8
;;       xload64le_offset32 x8, x0, 104
;;       br_if_xult64 x8, x7, 0x17    // target = 0x2d
;;   1d: xload64le_offset32 x8, x0, 96
;;       xload32le_offset32 x0, x8, 131068
;;       pop_frame
;;       ret
;;   2d: trap
;;
;; wasm[0]::function[9]::never_inbounds:
;;       push_frame
;;       xload64le_offset32 x7, x0, 104
;;       xconst8 x8, 4
;;       xsub64 x7, x7, x8
;;       xconst32 x8, 131069
;;       br_if_xult64 x7, x8, 0x17    // target = 0x2b
;;   1b: xload64le_offset32 x8, x0, 96
;;       xload32le_offset32 x0, x8, 131069
;;       pop_frame
;;       ret
;;   2b: trap
;;
;; wasm[0]::function[10]::never_inbounds_v2:
;;       push_frame
;;       trap
