;; basic function lifting
(component
  (core module $m
    (func (export ""))
  )
  (core instance $i (instantiate $m))

  (func (export "thunk")
    (canon lift (core func $i ""))
  )
)

;; use an aliased type
(component $c
  (core module $m
    (func (export ""))
  )
  (core instance $i (instantiate $m))

  (type $to_alias (func))
  (alias outer $c $to_alias (type $alias))

  (func (export "thunk") (type $alias)
    (canon lift (core func $i ""))
  )
)

;; test out some various canonical abi
(component $c
  (core module $m
    (func (export "") (param i32 i32))
    (memory (export "memory") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      unreachable)
  )
  (core instance $i (instantiate $m))

  (func (export "thunk") (param string)
    (canon lift
      (core func $i "")
      (memory $i "memory")
      (realloc (func $i "realloc"))
    )
  )

  (func (export "thunk8") (param string)
    (canon lift
      (core func $i "")
      string-encoding=utf8
      (memory $i "memory")
      (realloc (func $i "realloc"))
    )
  )

  (func (export "thunk16") (param string)
    (canon lift
      (core func $i "")
      string-encoding=utf16
      (memory $i "memory")
      (realloc (func $i "realloc"))
    )
  )

  (func (export "thunklatin16") (param string)
    (canon lift
      (core func $i "")
      string-encoding=latin1+utf16
      (memory $i "memory")
      (realloc (func $i "realloc"))
    )
  )
)

;; lower something then immediately lift it
(component $c
  (import "host-return-two" (func $f (result u32)))

  (core func $f_lower
    (canon lower (func $f))
  )
  (func $f2 (result s32)
    (canon lift (core func $f_lower))
  )
  (export "f" (func $f2))
)
