;; Calls the preload's `async func`. The caller is async-lifted too, since a
;; sync-lifted caller cannot block on an async-lowered import.
(component
  (type $t (func async))
  (import "dep" (instance $dep
    (export "dep" (instance (export "go" (func (type $t)))))))
  (alias export $dep "dep" (instance $d))
  (core func $go (canon lower (func $d "go")))
  (core module $m
    (import "" "go" (func $go))
    (func (export "run") call $go))
  (core instance $i (instantiate $m
    (with "" (instance (export "go" (func $go))))))
  (func (export "run") (type $t) (canon lift (core func $i "run")))
)
