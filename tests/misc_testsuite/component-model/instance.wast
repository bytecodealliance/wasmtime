(component
  (core module $m)
  (core instance (instantiate $m))
)

(component
  (core module $m
    (func (export ""))
  )
  (core instance $i (instantiate $m))

  (core module $m2
    (func (import "" ""))
  )
  (core instance (instantiate $m2 (with "" (instance $i))))
)

(component
  (core module $m
    (func (export "a"))
  )
  (core instance $i (instantiate $m))

  (core module $m2
    (func (import "" "b"))
  )
  (core instance (instantiate $m2
    (with "" (instance (export "b" (func $i "a"))))
  ))
)

;; all kinds of imports for core wasm modules, and register a start function on
;; one module to ensure that everything is correct
(component
  (core module $m
    (func (export "a"))
    (table (export "b") 1 funcref)
    (memory (export "c") 1)
    (global (export "d") i32 i32.const 1)
  )
  (core instance $i (instantiate $m))

  (core module $m2
    (import "" "a" (func $f))
    (import "" "b" (table 1 funcref))
    (import "" "c" (memory 1))
    (import "" "d" (global $g i32))

    (func $start
      global.get $g
      i32.const 1
      i32.ne
      if
        unreachable
      end

      call $f
    )

    (start $start)

    (data (i32.const 0) "hello")
    (elem (i32.const 0) $start)
  )
  (core instance (instantiate $m2
    (with "" (instance $i))
  ))
)

;; double-check the start function runs by ensuring that a trap shows up and it
;; sees the wrong value for the global import
(assert_trap
  (component
    (core module $m
      (global (export "g") i32 i32.const 1)
    )
    (core instance $i (instantiate $m))

    (core module $m2
      (import "" "g" (global $g i32))

      (func $start
        global.get $g
        i32.const 0
        i32.ne
        if
          unreachable
        end
      )

      (start $start)
    )
    (core instance (instantiate $m2 (with "" (instance $i))))
  )
  "unreachable")

;; shuffle around imports to get to what the target core wasm module needs
(component
  (core module $m
    (func (export "1"))
    (table (export "2") 1 funcref)
    (memory (export "3") 1)
    (global (export "4") i32 i32.const 1)
  )
  (core instance $i (instantiate $m))

  (core module $m2
    (import "" "a" (func $f))
    (import "" "b" (table 1 funcref))
    (import "" "c" (memory 1))
    (import "" "d" (global $g i32))
  )
  (core instance (instantiate $m2
    (with "" (instance
      (export "a" (func $i "1"))
      (export "b" (table $i "2"))
      (export "c" (memory $i "3"))
      (export "d" (global $i "4"))
    ))
  ))
)

;; indirect references through a synthetic instance
(component
  (core module $m
    (func (export "a"))
    (table (export "b") 1 funcref)
    (memory (export "c") 1)
    (global (export "d") i32 i32.const 1)
  )
  (core instance $i (instantiate $m))
  (core instance $i2
    (export "a1" (func $i "a"))
    (export "a2" (table $i "b"))
    (export "a3" (memory $i "c"))
    (export "a4" (global $i "d"))
  )

  (core module $m2
    (import "" "1" (func $f))
    (import "" "2" (table 1 funcref))
    (import "" "3" (memory 1))
    (import "" "4" (global $g i32))
  )
  (core instance (instantiate $m2
    (with "" (instance
      (export "1" (func $i2 "a1"))
      (export "2" (table $i2 "a2"))
      (export "3" (memory $i2 "a3"))
      (export "4" (global $i2 "a4"))
    ))
  ))
)

(component
  (import "host" (instance $i (export "return-three" (func (result u32)))))

  (core module $m
    (import "host" "return-three" (func $three (result i32)))
    (func $start
      call $three
      i32.const 3
      i32.ne
      if unreachable end
    )
    (start $start)
  )
  (core func $three_lower
    (canon lower (func $i "return-three"))
  )
  (core instance (instantiate $m
    (with "host" (instance (export "return-three" (func $three_lower))))
  ))
)

(component
  (import "host" (instance $i
    (export "nested" (instance
      (export "return-four" (func (result u32)))
    ))
  ))

  (core module $m
    (import "host" "return-three" (func $three (result i32)))
    (func $start
      call $three
      i32.const 4
      i32.ne
      if unreachable end
    )
    (start $start)
  )
  (core func $three_lower
    (canon lower (func $i "nested" "return-four"))
  )
  (core instance (instantiate $m
    (with "host" (instance (export "return-three" (func $three_lower))))
  ))
)

(component
  (import "host" (instance $i
    (export "simple-module" (core module))
  ))

  (core instance (instantiate (module $i "simple-module")))
)

(component
  (import "host" (instance $i
    (export "simple-module" (core module
      (export "f" (func (result i32)))
      (export "g" (global i32))
    ))
  ))

  (core instance $i (instantiate (module $i "simple-module")))
  (core module $verify
    (import "host" "f" (func $f (result i32)))
    (import "host" "g" (global $g i32))

    (func $start
      call $f
      i32.const 101
      i32.ne
      if unreachable end

      global.get $g
      i32.const 100
      i32.ne
      if unreachable end
    )
    (start $start)
  )

  (core instance (instantiate $verify (with "host" (instance $i))))
)

;; export an instance
(component
  (core module $m)
  (instance $i (export "m" (core module $m)))
  (export "i" (instance $i))
)
(component
  (component $c)
  (instance $i (instantiate $c))
  (export "i" (instance $i))
)
(component
  (import "host" (instance $i))
  (export "i" (instance $i))
)

