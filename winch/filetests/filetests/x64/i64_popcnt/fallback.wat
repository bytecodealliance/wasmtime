;;! target = "x86_64"

(module
    (func (result i64)
      i64.const 15
      i64.popcnt
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c00f000000       	mov	rax, 0xf
;;   13:	 4889c1               	mov	rcx, rax
;;   16:	 48c1e801             	shr	rax, 1
;;   1a:	 49bb5555555555555555 	
;; 				movabs	r11, 0x5555555555555555
;;   24:	 4c21d8               	and	rax, r11
;;   27:	 4829c1               	sub	rcx, rax
;;   2a:	 4889c8               	mov	rax, rcx
;;   2d:	 49bb3333333333333333 	
;; 				movabs	r11, 0x3333333333333333
;;   37:	 4c21d8               	and	rax, r11
;;   3a:	 48c1e902             	shr	rcx, 2
;;   3e:	 4c21d9               	and	rcx, r11
;;   41:	 4801c1               	add	rcx, rax
;;   44:	 4889c8               	mov	rax, rcx
;;   47:	 48c1e804             	shr	rax, 4
;;   4b:	 4801c8               	add	rax, rcx
;;   4e:	 49bb0f0f0f0f0f0f0f0f 	
;; 				movabs	r11, 0xf0f0f0f0f0f0f0f
;;   58:	 4c21d8               	and	rax, r11
;;   5b:	 49bb0101010101010101 	
;; 				movabs	r11, 0x101010101010101
;;   65:	 490fafc3             	imul	rax, r11
;;   69:	 48c1e838             	shr	rax, 0x38
;;   6d:	 4883c408             	add	rsp, 8
;;   71:	 5d                   	pop	rbp
;;   72:	 c3                   	ret	
