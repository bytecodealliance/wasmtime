Main (8c11e2e0edbd344886b6daab87aba5eb3a6e5ca9):
```
wasi-http/p3_http_proxy time:   [17.731 µs 17.751 µs 17.780 µs]
Found 5 outliers among 100 measurements (5.00%)
  1 (1.00%) high mild
  4 (4.00%) high severe

wasi-http/p3-api/request.new
                        time:   [1.4515 µs 1.4536 µs 1.4561 µs]
Found 2 outliers among 100 measurements (2.00%)
  1 (1.00%) high mild
  1 (1.00%) high severe

wasi-http/p3-api/response.consume-body
                        time:   [2.1158 µs 2.1311 µs 2.1449 µs]
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high severe

wasi-http/p3-api/client.send
                        time:   [2.9556 µs 2.9614 µs 2.9667 µs]
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high mild

```

Branch (a20f75b8a0fb160653fca8421119721a8cb1fec3):
```
wasi-http/p3_http_proxy time:   [16.834 µs 16.858 µs 16.890 µs]
                        change: [−5.2232% −5.0299% −4.8189%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 5 outliers among 100 measurements (5.00%)
  1 (1.00%) high mild
  4 (4.00%) high severe

wasi-http/p3-api/request.new
                        time:   [1.1689 µs 1.1733 µs 1.1805 µs]
                        change: [−19.787% −19.146% −18.210%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 8 outliers among 100 measurements (8.00%)
  2 (2.00%) low mild
  4 (4.00%) high mild
  2 (2.00%) high severe

wasi-http/p3-api/response.consume-body
                        time:   [1.7009 µs 1.7146 µs 1.7300 µs]
                        change: [−19.751% −18.733% −17.645%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high severe

wasi-http/p3-api/client.send
                        time:   [2.9678 µs 2.9714 µs 2.9746 µs]
                        change: [+0.1770% +0.5625% +0.9582%] (p = 0.01 < 0.05)
                        Change within noise threshold.
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high mild
```
