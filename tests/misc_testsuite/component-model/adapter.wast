;; basic function lifting
(component
  (module $m
    (func (export ""))
  )
  (instance $i (instantiate (module $m)))

  (func (export "thunk")
    (canon.lift (func) (func $i ""))
  )
)

;; use an aliased type
(component $c
  (module $m
    (func (export ""))
  )
  (instance $i (instantiate (module $m)))

  (type $to_alias (func))
  (alias outer $c $to_alias (type $alias))

  (func (export "thunk")
    (canon.lift (type $alias) (func $i ""))
  )
)

;; test out some various canonical abi
(component $c
  (module $m
    (func (export "") (param i32 i32))
    (memory (export "memory") 1)
    (func (export "canonical_abi_realloc") (param i32 i32 i32 i32) (result i32)
      unreachable)
    (func (export "canonical_abi_free") (param i32 i32 i32))
  )
  (instance $i (instantiate (module $m)))

  (func (export "thunk")
    (canon.lift (func (param string)) (into $i) (func $i ""))
  )

  (func (export "thunk8")
    (canon.lift (func (param string)) string=utf8 (into $i) (func $i ""))
  )

  (func (export "thunk16")
    (canon.lift (func (param string)) string=utf16 (into $i) (func $i ""))
  )

  (func (export "thunklatin16")
    (canon.lift (func (param string)) string=latin1+utf16 (into $i) (func $i ""))
  )
)
