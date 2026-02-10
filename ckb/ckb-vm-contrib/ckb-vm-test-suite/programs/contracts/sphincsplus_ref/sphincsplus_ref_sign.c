#include <stdint.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

#include "api.h"
#include "params.h"
#include "randombytes.h"

#define SPX_MLEN 32

int main() {
    uint8_t pk[SPX_PK_BYTES];
    uint8_t sk[SPX_SK_BYTES];
    uint8_t m[SPX_MLEN];
    uint8_t sm[SPX_BYTES + SPX_MLEN];
    uint8_t mout[SPX_BYTES + SPX_MLEN];
    unsigned long long smlen;
    unsigned long long mlen;

    randombytes(m, SPX_MLEN);
    if (crypto_sign_keypair(pk, sk)) {
        return 1;
    }
    crypto_sign(sm, &smlen, m, SPX_MLEN, sk);
    if (smlen != SPX_BYTES + SPX_MLEN) {
        return 1;
    }
    if (crypto_sign_open(mout, &mlen, sm, smlen, pk)) {
        return 1;
    }
    if (mlen != SPX_MLEN) {
        return 1;
    }
    if (memcmp(m, mout, SPX_MLEN)) {
        return 1;
    }

    printf("#include <stdint.h>");
    printf("\n");
    printf("\n");

    printf("uint8_t sm[%llu] = {", smlen);
    printf("\n");
    for (int i = 0; i < smlen; i++) {
        if ((i + 1) % 16 == 1) {
            printf("    ");
        }
        printf("0x%02x,", sm[i]);
        if ((i + 1) % 16 == 0) {
            printf("\n");
        }
    }
    printf("};");
    printf("\n");
    printf("\n");

    printf("uint8_t pk[%u] = {", SPX_PK_BYTES);
    printf("\n");
    for (int i = 0; i < SPX_PK_BYTES; i++) {
        if ((i + 1) % 16 == 1) {
            printf("    ");
        }
        printf("0x%02x,", pk[i]);
        if ((i + 1) % 16 == 0) {
            printf("\n");
        }
    }
    printf("};");
    printf("\n");
    printf("\n");

    printf("uint8_t m[%u] = {", SPX_MLEN);
    printf("\n");
    for (int i = 0; i < SPX_MLEN; i++) {
        if ((i + 1) % 16 == 1) {
            printf("    ");
        }
        printf("0x%02x,", m[i]);
        if ((i + 1) % 16 == 0) {
            printf("\n");
        }
    }
    printf("};");
    printf("\n");

    return 0;
}
