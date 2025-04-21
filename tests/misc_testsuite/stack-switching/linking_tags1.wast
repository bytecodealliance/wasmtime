;;! stack_switching = true
(module $alien
  (tag $alien_tag (export "alien_tag"))
)
(register "alien")

(module $mine
  (type $ft (func))
  (type $ct (cont $ft))
  (tag $alien_tag (import "alien" "alien_tag"))
  (tag $my_tag)
  (func $do_alien_tag
    (suspend $alien_tag))

  ;; Don't handle the imported alien.
  (func (export "main-1")
    (block $on_my_tag (result (ref $ct))
      (resume $ct (on $my_tag $on_my_tag) (cont.new $ct (ref.func $do_alien_tag)))
      (unreachable)
    )
    (unreachable))

  ;; Handle the imported alien.
  (func (export "main-2")
    (block $on_alien_tag (result (ref $ct))
      (resume $ct (on $alien_tag $on_alien_tag) (cont.new $ct (ref.func $do_alien_tag)))
      (unreachable)
    )
    (drop))

  (elem declare func $do_alien_tag)
)
(register "mine")
(assert_return (invoke "main-2"))
;; Due to issue #253, we need to make sure that nothing happens afterwards in
;; the test:
(assert_suspension (invoke "main-1") "unhandled")
