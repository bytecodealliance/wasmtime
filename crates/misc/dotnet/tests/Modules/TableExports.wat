(module
  (table $t0 1 10 funcref)
  (table $t1 10 anyref)
  (table $t2 100 1000 funcref)
  (export "table1" (table $t0))
  (export "table2" (table $t1))
  (export "table3" (table $t2))
)
