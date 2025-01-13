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
;;       0x10    // target = 0x17
;;       0x8    // target = 0x13
;;       0xd    // target = 0x1c
;;   13: xone x0
;;       pop_frame
;;       ret
;;   17: xconst8 x0, 2
;;   1a: pop_frame
;;   1b: ret
;;   1c: xzero x0
;;   1e: pop_frame
;;   1f: ret
