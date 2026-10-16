;; A `--preload` exporting an `async func`, which must be forwarded with
;; `func_new_concurrent` rather than `func_new`.
(component
  (type $t (func async))
  (core module $m (func (export "go")))
  (core instance $i (instantiate $m))
  (func $go (type $t) (canon lift (core func $i "go")))
  (instance $inst (export "go" (func $go)))
  (export "dep" (instance $inst))
)
