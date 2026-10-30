;;! bulk_memory = true
;;! exceptions = true
;;! function_references = true
;;! stack_switching = true

;; Two instances of the same module have distinct tags, so a handler installed
;; by one instance doesn't handle a suspension to the other's tag.
(module definition $other
  (type $ft (func))
  (type $ct (cont $ft))
  (tag $tag)

  (func (export "suspend")
    (suspend $tag))

  (func (export "resume") (param $f (ref $ct))
    (block $handler (result (ref $ct))
      (resume $ct (on $tag $handler) (local.get $f))
      (unreachable))
    (drop))
)
(module instance $other1 $other)
(module instance $other2 $other)
(register "other1" $other1)
(register "other2" $other2)

(module
  (type $ft (func))
  (type $ct (cont $ft))

  (import "other1" "suspend" (func $suspend))
  (import "other2" "resume" (func $resume (param (ref $ct))))

  (elem declare func $suspend)

  (func (export "entry")
    (call $resume (cont.new $ct (ref.func $suspend))))
)

;; Due to issue #253, we need to make sure that nothing happens afterwards in
;; the test:
(assert_suspension (invoke "entry") "unhandled")
