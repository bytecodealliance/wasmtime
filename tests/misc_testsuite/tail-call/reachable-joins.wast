;;! tail_call = true
;;! reference_types = true

(module
  (func (export "leaf") (result i32) i32.const 42))
(register "provider")

;; Each taken branch exits, but compilation continues at the join. There are
;; enough branches to exhaust the registers if temporary callee registers leak.
(module
  (type $t (func (result i32)))
  (import "provider" "leaf" (func $leaf (type $t)))
  (table funcref (elem $leaf))
  (func (export "direct") (param i32) (result i32)
    local.get 0 i32.const 0 i32.eq if return_call $leaf end
    local.get 0 i32.const 1 i32.eq if return_call $leaf end
    local.get 0 i32.const 2 i32.eq if return_call $leaf end
    local.get 0 i32.const 3 i32.eq if return_call $leaf end
    local.get 0 i32.const 4 i32.eq if return_call $leaf end
    local.get 0 i32.const 5 i32.eq if return_call $leaf end
    local.get 0 i32.const 6 i32.eq if return_call $leaf end
    local.get 0 i32.const 7 i32.eq if return_call $leaf end
    local.get 0 i32.const 8 i32.eq if return_call $leaf end
    local.get 0 i32.const 9 i32.eq if return_call $leaf end
    local.get 0 i32.const 10 i32.eq if return_call $leaf end
    local.get 0 i32.const 11 i32.eq if return_call $leaf end
    local.get 0 i32.const 12 i32.eq if return_call $leaf end
    local.get 0 i32.const 13 i32.eq if return_call $leaf end
    local.get 0 i32.const 14 i32.eq if return_call $leaf end
    local.get 0 i32.const 15 i32.eq if return_call $leaf end
    local.get 0 i32.const 16 i32.eq if return_call $leaf end
    local.get 0 i32.const 17 i32.eq if return_call $leaf end
    local.get 0 i32.const 18 i32.eq if return_call $leaf end
    local.get 0 i32.const 19 i32.eq if return_call $leaf end
    local.get 0 i32.const 20 i32.eq if return_call $leaf end
    local.get 0 i32.const 21 i32.eq if return_call $leaf end
    local.get 0 i32.const 22 i32.eq if return_call $leaf end
    local.get 0 i32.const 23 i32.eq if return_call $leaf end
    local.get 0 i32.const 24 i32.eq if return_call $leaf end
    local.get 0 i32.const 25 i32.eq if return_call $leaf end
    local.get 0 i32.const 26 i32.eq if return_call $leaf end
    local.get 0 i32.const 27 i32.eq if return_call $leaf end
    local.get 0 i32.const 28 i32.eq if return_call $leaf end
    local.get 0 i32.const 29 i32.eq if return_call $leaf end
    local.get 0 i32.const 30 i32.eq if return_call $leaf end
    local.get 0 i32.const 31 i32.eq if return_call $leaf end
    i32.const -1)
  (func (export "indirect") (param i32) (result i32)
    local.get 0 i32.const 0 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 1 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 2 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 3 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 4 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 5 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 6 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 7 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 8 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 9 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 10 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 11 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 12 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 13 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 14 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 15 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 16 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 17 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 18 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 19 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 20 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 21 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 22 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 23 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 24 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 25 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 26 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 27 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 28 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 29 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 30 i32.eq if i32.const 0 return_call_indirect (type $t) end
    local.get 0 i32.const 31 i32.eq if i32.const 0 return_call_indirect (type $t) end
    i32.const -1)
)

(assert_return (invoke "direct" (i32.const 0)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 1)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 2)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 3)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 4)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 5)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 6)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 7)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 8)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 9)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 10)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 11)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 12)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 13)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 14)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 15)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 16)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 17)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 18)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 19)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 20)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 21)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 22)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 23)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 24)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 25)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 26)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 27)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 28)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 29)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 30)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 31)) (i32.const 42))
(assert_return (invoke "direct" (i32.const 32)) (i32.const -1))

(assert_return (invoke "indirect" (i32.const 0)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 1)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 2)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 3)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 4)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 5)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 6)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 7)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 8)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 9)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 10)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 11)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 12)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 13)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 14)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 15)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 16)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 17)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 18)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 19)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 20)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 21)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 22)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 23)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 24)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 25)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 26)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 27)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 28)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 29)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 30)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 31)) (i32.const 42))
(assert_return (invoke "indirect" (i32.const 32)) (i32.const -1))
