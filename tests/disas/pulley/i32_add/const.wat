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
;;       xconst8 spilltmp0, -16
;;       xadd32 sp, sp, spilltmp0
;;       store64_offset8 sp, 8, lr
;;       store64 sp, fp
;;       xmov fp, sp
;;       xconst8 x0, 30
;;       load64_offset8 lr, sp, 8
;;       load64 fp, sp
;;       xconst8 spilltmp0, 16
;;       xadd32 sp, sp, spilltmp0
;;       ret
