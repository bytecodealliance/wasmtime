;;! target = "pulley64"
;;! test = "compile"

(module
    (func (result i32)
        i32.const 10
        i32.const 20
        i32.add
    )
)

;; wasm[0]::function[0]:
;;       push_frame
;;       xconst8 x0, 30
;;       pop_frame
;;       ret
