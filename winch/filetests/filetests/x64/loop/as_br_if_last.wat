;;! target = "x86_64"
(module
  (func (export "as-br-if-last") (result i32)
    (block (result i32) (br_if 0 (i32.const 2) (loop (result i32) (i32.const 1))))
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8720000000         	ja	0x3b
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b901000000           	mov	ecx, 1
;;      	 b802000000           	mov	eax, 2
;;      	 85c9                 	test	ecx, ecx
;;      	 0f8500000000         	jne	0x35
;;   35:	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3b:	 0f0b                 	ud2	
