;;! target = "x86_64"
(module
    (func (result i64)
	(i64.const 1)
	(i64.const 0x7fffffffffffffff)
	(i64.add)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 48c7c001000000       	mov	rax, 1
;;    b:	 49bbffffffffffffff7f 	
;; 				movabs	r11, 0x7fffffffffffffff
;;   15:	 4c01d8               	add	rax, r11
;;   18:	 5d                   	pop	rbp
;;   19:	 c3                   	ret	
