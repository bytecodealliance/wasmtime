;; Reachable `if` head and consequent and unreachable alternative means that the
;; following block is also reachable.

(module
  (func (param i32) (result i32)
    local.get 0
    if
      nop
    else
      unreachable
    end
    i32.const 0))
