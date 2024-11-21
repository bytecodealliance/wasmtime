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
;;       push_frame
;;       br_table32 x2, 3
;;       0x11    // target = 0x18
;;       0x8    // target = 0x13
;;       0xe    // target = 0x1d
;;   13: xconst8 x0, 1
;;       pop_frame
;;       ret
;;   18: xconst8 x0, 2
;;   1b: pop_frame
;;   1c: ret
;;   1d: xconst8 x0, 0
;;   20: pop_frame
;;   21: ret
