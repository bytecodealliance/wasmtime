;;! target = "x86_64"
(module
  (func $dummy)
  (func (export "as-if-condition")
    (loop (result i32) (i32.const 1)) (if (then (call $dummy)))
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883c408             	add	rsp, 8
;;   10:	 5d                   	pop	rbp
;;   11:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b801000000           	mov	eax, 1
;;   11:	 85c0                 	test	eax, eax
;;   13:	 0f840d000000         	je	0x26
;;   19:	 4883ec08             	sub	rsp, 8
;;   1d:	 e800000000           	call	0x22
;;   22:	 4883c408             	add	rsp, 8
;;   26:	 4883c408             	add	rsp, 8
;;   2a:	 5d                   	pop	rbp
;;   2b:	 c3                   	ret	
