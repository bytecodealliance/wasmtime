(module
  (table $t 1 externref)

  (func (export "init") (param externref)
    (table.set $t (i32.const 0) (local.get 0))
  )

  (func (export "get-many-externrefs") (param $i i32)
    (loop $continue
      ;; Exit when our loop counter `$i` reaches zero.
      (if (i32.eqz (local.get $i))
        (return)
      )

      ;; Get an `externref` out of the table. This could cause the
      ;; `VMExternRefActivationsTable`'s bump region to reach full capacity,
      ;; which triggers a GC.
      ;;
      ;; Set the table element back into the table, just so that the element is
      ;; still considered live at the time of the `table.get`, it ends up in the
      ;; stack map, and we poke more of our GC bits.
      (table.set $t (i32.const 0) (table.get $t (i32.const 0)))

      ;; Decrement our loop counter `$i`.
      (local.set $i (i32.sub (local.get $i) (i32.const 1)))

      ;; Continue to the next loop iteration.
      (br $continue)
    )
    unreachable
  )
)

(invoke "init" (ref.extern 1))
(invoke "get-many-externrefs" (i32.const 8192))
