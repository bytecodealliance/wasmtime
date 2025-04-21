;;! stack_switching = true
;; Test unhandled suspension

(module
  (type $ft (func))
  (type $ct (cont $ft))
  (tag $t)

  (func $suspend
    (suspend $t))
  (elem declare func $suspend)

  (func $unhandled-0 (export "unhandled-0")
    (call $suspend))

  (func $unhandled-1 (export "unhandled-1")
    (resume $ct (cont.new $ct (ref.func $suspend))))
)

;; TODO(dhil): Suspending on the main thread currently causes an
;; unrecoverable panic. Instead we should emit the UnhandledTrap trap
;; code; once this has been implemented the below test should pass.
;;(assert_suspension (invoke "unhandled-0") "unhandled")
(assert_suspension (invoke "unhandled-1") "unhandled")