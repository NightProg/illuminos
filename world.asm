global _start
_start:
    mov rax, 2
    mov rdi, 0
    lea rsi, [world]
    mov rdx, world_len
    syscall
    mov rax, 0
    syscall
    jmp $

section .data
world: db "World", 10
world_len: equ $-world
