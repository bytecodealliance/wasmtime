#include <stdio.h>
#include <stdint.h>

uint8_t clz(int64_t x);
uint8_t cls(int64_t x);

uint8_t clz(int64_t x) {
    int64_t y;
    uint8_t num_zeros = 0;
    
    y = x >> 32;
    if (y != 0) x = y; else num_zeros += 32;
    
    y = x >> 16;
    if (y != 0) x = y; else num_zeros += 16;

    y = x >> 8;
    if (y != 0) x = y; else num_zeros += 8;

    y = x >> 4;
    if (y != 0) x = y; else num_zeros += 4;

    y = x >> 2;
    if (y != 0) x = y; else num_zeros += 2;

    y = x >> 1;
    if (y != 0) x = y; else num_zeros += 1;

    if (x == 0) num_zeros += 1;

    return num_zeros;
}

uint8_t cls(int64_t x) {
    uint8_t sign_bits = 0;
    
    if (x >= 0) {
    if (0 <= x)
        sign_bits = clz(x);
        return sign_bits == 0 ? sign_bits : sign_bits - 1;
    }
    
    int64_t y;
    
    y = x >> 32;
    if (y != -1) x = y; else sign_bits += 32;
    
    y = x >> 16;
    if (y != -1) x = y; else sign_bits += 16;

    y = x >> 8;
    if (y != -1) x = y; else sign_bits += 8;

    y = x >> 4;
    if (y != -1) x = y; else sign_bits += 4;

    y = x >> 2;
    if (y != -1) x = y; else sign_bits += 2;

    y = x >> 1;
    if (y != -1) x = y; else sign_bits += 1;

    if (x == -1) sign_bits += 1;

    return sign_bits == 0 ? sign_bits : sign_bits - 1;
}

int main()
{
    printf("cls(0) = %d\n", cls(0));
    for (uint64_t i = 0; i < 64; i++) {
        printf("cls(%#llx) = %d\n", 1ULL << i, cls(1ULL << i));
    }
    
    printf("\n");
    
    printf("cls(-1) = %d\n", cls(-1));
    for (int i = 0; i < 64; i++) {
        printf("cls(%#llx) = %d\n", ~(1ULL << i), cls(~(1ULL << i)));
    }
    
    return 0;
}