;;! target = "pulley32"
;;! test = "compile"

(module
  (import "" "" (func $x))
  (func (export "") call $x)
)
;; wasm[0]::function[1]:
;;       push_frame
;;       xload32le_o32 x3, x0, 28
;;       xload32le_o32 x4, x0, 36
;;       call_indirect2 x3, x4, x0
;;       pop_frame
;;       ret
