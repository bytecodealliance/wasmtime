;;! target = "pulley32"
;;! test = "compile"

(module
  (import "" "" (func $x))
  (func (export "") call $x)
)
;; wasm[0]::function[1]:
;;       push_frame
;;       xload32le_offset8 x3, x0, 44
;;       xmov x6, x0
;;       xload32le_offset8 x0, x6, 52
;;       xmov x1, x6
;;       call_indirect x3
;;       pop_frame
;;       ret
