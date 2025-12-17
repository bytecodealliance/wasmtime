;;! exceptions = true

(component
  (core module $a (tag (export "t")))
  (core module $b (import "a" "t" (tag)))

  (core instance $a (instantiate $a))
  (core instance (instantiate $b (with "a" (instance $a))))
  (core instance (instantiate $b
    (with "a" (instance
      (export "t" (tag $a "t"))
    ))
  ))
)
