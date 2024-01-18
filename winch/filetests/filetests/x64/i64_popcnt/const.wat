;;! target = "x86_64"
;;! flags = ["has_popcnt", "has_sse42"]

(module
    (func (result i64)
      i64.const 3
      i64.popcnt
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8716000000         	ja	0x2e
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c003000000       	mov	rax, 3
;;      	 f3480fb8c0           	popcnt	rax, rax
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   2e:	 0f0b                 	ud2	
