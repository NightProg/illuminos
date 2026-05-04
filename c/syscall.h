#pragma once

#include <stddef.h>
#include <stdint.h>

#define SYS_EXIT  0
#define SYS_READ  1
#define SYS_WRITE 2
#define SYS_OPENF 3
#define SYS_YIELD 4
#define SYS_FB_INFO 5
#define SYS_FB_MMAP 6
#define SYS_FB_SWAP 7
#define SYS_SIGNAL 8
#define SYS_RAISE  9
#define SYS_FORK 10
#define SYS_GETPID 11

#define SIG_KILL   1
#define SIG_PAUSE  2
#define SIG_RESUME 3
#define SIG_PAGE_FAULT 4
#define SIG_USER1  20
#define SIG_USER2  21
#define SIG_USER3  22

#define O_READ   (1 << 0)
#define O_WRITE  (1 << 1)
#define O_CREATE (1 << 2)
#define O_TRUNC  (1 << 3)

#define FD_INVALID  UINT64_MAX
#define ERR_SYSCALL UINT64_MAX

typedef void (*signal_handler_t)(void);

typedef enum {
    PIXEL_FORMAT_RGB = 0,
    PIXEL_FORMAT_BGR = 1,
    PIXEL_FORMAT_U8 = 2,
} pixel_format_t;

typedef struct {
    uint64_t byte_len;
    uint64_t width;
    uint64_t height;
    uint32_t pixel_format;
    uint64_t bytes_per_pixel;
    uint64_t stride;
} fb_info_t;

typedef uint64_t fd_t;

static inline uint64_t _syscall3(uint64_t id,
                                  uint64_t a1,
                                  uint64_t a2,
                                  uint64_t a3)
{
    register uint64_t r9 asm("r9") = 0;
    asm volatile (
        "syscall"
        : "+r"(r9)
        : "a"(id), "D"(a1), "S"(a2), "d"(a3)
        : "rcx", "r11", "memory"
    );
    return r9;
}

static inline uint64_t _syscall0_ret(uint64_t id) {
    uint64_t ret;
    register uint64_t r9 asm("r9") = 0;
    asm volatile (
        "syscall"
        : "=a"(ret), "+r"(r9)
        : "a"(id)
        : "rcx", "r11", "memory"
    );
    return (r9 == ERR_SYSCALL) ? UINT64_MAX : ret;
}


#define _syscall0(id)     _syscall3((id), 0, 0, 0)
#define _syscall1(id,a)   _syscall3((id), (uint64_t)(a), 0, 0)
#define _syscall2(id,a,b) _syscall3((id), (uint64_t)(a), (uint64_t)(b), 0)

static inline void sys_exit(void)
{
    _syscall0(SYS_EXIT);
    __builtin_unreachable();
}


static inline void sys_yield(void)
{
    _syscall0(SYS_YIELD);
}

static inline int sys_write(fd_t fd, const void *buf, size_t len)
{
    uint64_t err = _syscall3(SYS_WRITE,
                             (uint64_t)fd,
                             (uint64_t)(uintptr_t)buf,
                             (uint64_t)len);
    return (err == ERR_SYSCALL) ? -1 : 0;
}


static inline int sys_read(fd_t fd, void *buf, size_t len)
{
    uint64_t err = _syscall3(SYS_READ,
                             (uint64_t)fd,
                             (uint64_t)(uintptr_t)buf,
                             (uint64_t)len);
    return (err == ERR_SYSCALL) ? -1 : 0;
}

static inline fd_t sys_openf(const char *path, uint32_t flags)
{
    fd_t fd = FD_INVALID;
    uint64_t err = _syscall3(SYS_OPENF,
                             (uint64_t)(uintptr_t)path,
                             (uint64_t)(uintptr_t)&fd,
                             (uint64_t)flags);
    return (err == ERR_SYSCALL) ? FD_INVALID : fd;
}

static inline int sys_signal(uint64_t signum, signal_handler_t handler) {
    uint64_t err = _syscall2(SYS_SIGNAL, signum, (uint64_t)handler);
    return (err == ERR_SYSCALL) ? -1 : 0;
}

static inline int sys_raise(uint64_t pid, uint64_t signum) {
    uint64_t err = _syscall2(SYS_RAISE, pid, signum);
    return (err == ERR_SYSCALL) ? -1 : 0;
}

static inline uint64_t sys_fork(void) {
    return _syscall0_ret(SYS_FORK);
}
static inline uint64_t sys_getpid(void) {
    uint64_t pid = 0;
    uint64_t err = _syscall1(SYS_GETPID, (uint64_t)(uintptr_t)&pid);
    return (err == ERR_SYSCALL) ? UINT64_MAX : pid;
}

static inline int sys_fb_info(fb_info_t *info)
{
    uint64_t err = _syscall1(SYS_FB_INFO, (uint64_t)(uintptr_t)info);
    return (err == ERR_SYSCALL) ? -1 : 0;
}


static inline void* sys_fb_mmap(void)
{
    void* addr = NULL;
    uint64_t err = _syscall1(SYS_FB_MMAP, (uint64_t)(uintptr_t)&addr);
    return (err == ERR_SYSCALL) ? NULL : addr;
}


static inline int sys_fb_swap(void)
{
    uint64_t err = _syscall0(SYS_FB_SWAP);
    return (err == ERR_SYSCALL) ? -1 : 0;
}
