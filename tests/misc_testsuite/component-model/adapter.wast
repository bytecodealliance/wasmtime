;;! multi_memory = true

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

  (func (export "thunk") (param "a" string)
    (canon lift
      (core func $i "")
      (memory $i "memory")
      (realloc (func $i "realloc"))
    )
  )

  (func (export "thunk8") (param "a" string)
    (canon lift
      (core func $i "")
      string-encoding=utf8
      (memory $i "memory")
      (realloc (func $i "realloc"))
    )
  )

  (func (export "thunk16") (param "a" string)
    (canon lift
      (core func $i "")
      string-encoding=utf16
      (memory $i "memory")
      (realloc (func $i "realloc"))
    )
  )

  (func (export "thunklatin16") (param "a" string)
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

;; valid, but odd
(component
  (core module $m (func (export "")))
  (core instance $m (instantiate $m))

  (func $f1 (canon lift (core func $m "")))
  (core func $f2 (canon lower (func $f1)))
)
(assert_trap
  (component
    (core module $m (func (export "")))
    (core instance $m (instantiate $m))

    (func $f1 (canon lift (core func $m "")))
    (core func $f2 (canon lower (func $f1)))

    (core module $m2
      (import "" "" (func $f))
      (func $start
        call $f)
      (start $start)
    )
    (core instance (instantiate $m2
      (with "" (instance (export "" (func $f2))))
    ))
  )
  "degenerate component adapter called")

;; fiddling with 0-sized lists
(component $c
  (core module $m
    (func (export "x") (param i32 i32))
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      i32.const -1)
    (memory (export "memory") 0)
  )
  (core instance $m (instantiate $m))
  (type $t' (result))
  (export $t "t" (type $t'))
  (func $f (param "a" (list $t))
    (canon lift
      (core func $m "x")
      (realloc (func $m "realloc"))
      (memory $m "memory")
    )
  )
  (export "empty-list" (func $f))
)
(assert_trap (invoke "empty-list" (list.const)) "realloc return: beyond end of memory")
