;; Reachable `if` head and unreachable consequent and alternative, but with a
;; branch out of the alternative, means that the following block is reachable.

(module
  (func (param i32 i32) (result i32)
    local.get 0
    if
      unreachable
    else
      local.get 1
      br_if 0
      unreachable
    end
    i32.const 0))
