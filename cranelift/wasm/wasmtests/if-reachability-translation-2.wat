;; Reachable `if` head and unreachable consequent and reachable alternative
;; means that the following block is also reachable.

(module
  (func (param i32) (result i32)
    local.get 0
    if
      unreachable
    else
      nop
    end
    i32.const 0))
