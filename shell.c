#include "c/syscall.h"

#include <stdarg.h>

void putstr(fd_t fd, const char* s) {
  int len = 0;
  while (s[len] != '\0') len++;
  sys_write(fd, s, len);
}

void fprint_int(fd_t fd, int n) {
    char buf[20];
    int i = 0;

    if (n == 0) {
        putstr(fd, "0");
        return;
    }

    if (n < 0) {
        putstr(fd, "-");
        n = -n;
    }

    while (n > 0) {
        buf[i++] = '0' + (n % 10);
        n /= 10;
    }

    for (int j = i - 1; j >= 0; j--) {
        char c[2] = {buf[j], 0};
        putstr(fd, c);
    }
}

/* print hex (for %p) */
void fprint_hex(fd_t fd, uint64_t n) {
    char buf[16];
    int i = 0;

    if (n == 0) {
        putstr(fd, "0x0");
        return;
    }

    while (n > 0) {
        int digit = n & 0xf;
        buf[i++] = (digit < 10) ? ('0' + digit) : ('a' + digit - 10);
        n >>= 4;
    }

    putstr(fd, "0x");

    for (int j = i - 1; j >= 0; j--) {
        char c[2] = {buf[j], 0};
        putstr(fd, c);
    }
}

void printf(fd_t fd, const char* fmt, ...) {
    va_list args;
    va_start(args, fmt);

    for (int i = 0; fmt[i]; i++) {
        if (fmt[i] == '%') {
            i++;

            switch (fmt[i]) {
                case 'd': {
                    int v = va_arg(args, int);
                    fprint_int(fd, v);
                    break;
                }
                case 's': {
                    const char* s = va_arg(args, const char*);
                    putstr(fd, s);
                    break;
                }
                case 'p': {
                    void* p = va_arg(args, void*);
                    fprint_hex(fd, (uint64_t)p);
                    break;
                }
                case '%': {
                    putstr(fd, "%");
                    break;
                }
                default:
                    break;
            }
        } else {
            char c[2] = {fmt[i], 0};
            putstr(fd, c);
        }
    }

    va_end(args);
}

int main(int argc, char* argv[]) {
  fd_t fd = sys_openf("/dev/tty", O_READ | O_WRITE);

  if (fd != FD_INVALID) {
    int q = 1;
    while(1) {
      sys_sleep(1000);
      printf(fd, "HELLO");
    }
  }

  return 0;
} 
