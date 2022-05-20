(component
  (module $m)
  (instance (instantiate (module $m)))
)

(component
  (module $m
    (func (export ""))
  )
  (instance $i (instantiate (module $m)))

  (module $m2
    (func (import "" ""))
  )
  (instance (instantiate (module $m2) (with "" (instance $i))))
)

(component
  (module $m
    (func (export "a"))
  )
  (instance $i (instantiate (module $m)))

  (module $m2
    (func (import "" "b"))
  )
  (instance (instantiate (module $m2)
    (with "" (instance (export "b" (func $i "a"))))
  ))
)

;; all kinds of imports for core wasm modules, and register a start function on
;; one module to ensure that everything is correct
(component
  (module $m
    (func (export "a"))
    (table (export "b") 1 funcref)
    (memory (export "c") 1)
    (global (export "d") i32 i32.const 1)
  )
  (instance $i (instantiate (module $m)))

  (module $m2
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
  (instance (instantiate (module $m2)
    (with "" (instance $i))
  ))
)

;; double-check the start function runs by ensuring that a trap shows up and it
;; sees the wrong value for the global import
(assert_trap
  (component
    (module $m
      (global (export "g") i32 i32.const 1)
    )
    (instance $i (instantiate (module $m)))

    (module $m2
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
    (instance (instantiate (module $m2) (with "" (instance $i))))
  )
  "unreachable")

;; shuffle around imports to get to what the target core wasm module needs
(component
  (module $m
    (func (export "1"))
    (table (export "2") 1 funcref)
    (memory (export "3") 1)
    (global (export "4") i32 i32.const 1)
  )
  (instance $i (instantiate (module $m)))

  (module $m2
    (import "" "a" (func $f))
    (import "" "b" (table 1 funcref))
    (import "" "c" (memory 1))
    (import "" "d" (global $g i32))
  )
  (instance (instantiate (module $m2)
    (with "" (instance
      (export "a" (func $i "1"))
      (export "b" (table $i "2"))
      (export "c" (memory $i "3"))
      (export "d" (global $i "4"))
    ))
  ))
)
