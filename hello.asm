
global _start
_start:
    mov rax, 3
    lea rdi, [coucou]
    mov rsi, 0
    mov rdx, 1
    syscall

    jmp end
    cmp r9, 0xff
    je .error

    mov rax, 1
    mov rdi, [fd]
    lea rsi, [content]
    mov rdx, 64
    syscall


    mov rax, 0
    syscall

.error:
    mov rax, 2
    mov rdi, 0
    lea rsi, [error]
    mov rdx, error_len
    syscall
    jmp end

end:
    mov rax, 0
    syscall


section .data
hello: db "Hello from nasm", 10
hello_len: equ $-hello
error: db "ERROR", 10
error_len: equ $-error
coucou: db "/coucou.txt", 0
section .bss
    content resb 64
    fd resq 1
