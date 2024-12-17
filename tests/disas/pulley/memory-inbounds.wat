;;! target = "pulley64"
;;! test = "compile"

(module
  (memory 1)

  (func (result i32)
    (i32.const 0)
    i32.load
  )

  (func (result i32)
    (i32.const 100)
    i32.load
  )
)
;; wasm[0]::function[0]:
;;       push_frame
;;       xload64le_offset32 x3, x0, 96
;;       xload32le_offset32 x0, x3, 0
;;       pop_frame
;;       ret
;;
;; wasm[0]::function[1]:
;;       push_frame
;;       xload64le_offset32 x5, x0, 96
;;       xconst8 x6, 100
;;       xadd64 x5, x5, x6
;;       xload32le_offset32 x0, x5, 0
;;       pop_frame
;;       ret
