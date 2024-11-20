;;! target = "pulley64"
;;! test = "compile"

(module
    (func (param i32) (result i32)
      block $a
        block $b
          block $c
            local.get 0
            br_table $a $b $c
          end
          i32.const 0
          return
        end
        i32.const 1
        return
      end
      i32.const 2
    )
)
;; wasm[0]::function[0]:
;;       xconst8 spilltmp0, -16
;;       xadd32 sp, sp, spilltmp0
;;       store64_offset8 sp, 8, lr
;;       store64 sp, fp
;;       xmov fp, sp
;;       br_table32 x2, 3
;;       0x1d    // target = 0x33
;;       0x8    // target = 0x22
;;       0x26    // target = 0x44
;;   22: xconst8 x0, 1
;;       load64_offset8 lr, sp, 8
;;       load64 fp, sp
;;       xconst8 spilltmp0, 16
;;       xadd32 sp, sp, spilltmp0
;;       ret
;;   33: xconst8 x0, 2
;;   36: load64_offset8 lr, sp, 8
;;   3a: load64 fp, sp
;;   3d: xconst8 spilltmp0, 16
;;   40: xadd32 sp, sp, spilltmp0
;;   43: ret
;;   44: xconst8 x0, 0
;;   47: load64_offset8 lr, sp, 8
;;   4b: load64 fp, sp
;;   4e: xconst8 spilltmp0, 16
;;   51: xadd32 sp, sp, spilltmp0
;;   54: ret
