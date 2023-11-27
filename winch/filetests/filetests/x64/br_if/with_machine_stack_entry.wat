;;! target = "x86_64"

(module
  (func (export "") 
    call 1
    call 1
    br_if 0
    drop
  )
  (func (;1;) (result i32)
    i32.const 1
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883ec08             	sub	rsp, 8
;;   10:	 e800000000           	call	0x15
;;   15:	 4883c408             	add	rsp, 8
;;   19:	 4883ec04             	sub	rsp, 4
;;   1d:	 890424               	mov	dword ptr [rsp], eax
;;   20:	 4883ec04             	sub	rsp, 4
;;   24:	 e800000000           	call	0x29
;;   29:	 4883c404             	add	rsp, 4
;;   2d:	 85c0                 	test	eax, eax
;;   2f:	 0f8409000000         	je	0x3e
;;   35:	 4883c404             	add	rsp, 4
;;   39:	 e904000000           	jmp	0x42
;;   3e:	 4883c404             	add	rsp, 4
;;   42:	 4883c408             	add	rsp, 8
;;   46:	 5d                   	pop	rbp
;;   47:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b801000000           	mov	eax, 1
;;   11:	 4883c408             	add	rsp, 8
;;   15:	 5d                   	pop	rbp
;;   16:	 c3                   	ret	
