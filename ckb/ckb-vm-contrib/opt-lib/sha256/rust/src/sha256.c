/*************************** HEADER FILES ***************************/
#include "sha256.h"

/****************************** MACROS ******************************/
#define ROTRIGHT(a, b) (((a) >> (b)) | ((a) << (32 - (b))))

#define CH(x, y, z) (((x) & (y)) ^ (~(x) & (z)))
#define MAJ(x, y, z) ((((x) | (y)) & (z)) | ((x) & (y)))
#define EP0(x) (ROTRIGHT(x, 2) ^ ROTRIGHT(x, 13) ^ ROTRIGHT(x, 22))
#define EP1(x) (ROTRIGHT(x, 6) ^ ROTRIGHT(x, 11) ^ ROTRIGHT(x, 25))
#define SIG0(x) (ROTRIGHT(x, 7) ^ ROTRIGHT(x, 18) ^ ((x) >> 3))
#define SIG1(x) (ROTRIGHT(x, 17) ^ ROTRIGHT(x, 19) ^ ((x) >> 10))

/**************************** VARIABLES *****************************/
static const SHA256_WORD GLOBAL_K[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
    0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
    0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
    0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2};

/*********************** FUNCTION DEFINITIONS ***********************/
#define SHA256_ROUND(a, b, c, d, e, f, g, h, ki, mi)               \
    do {                                                           \
        SHA256_WORD t1 = (h) + EP1(e) + CH(e, f, g) + (ki) + (mi); \
        SHA256_WORD t2 = EP0(a) + MAJ(a, b, c);                    \
        (d) += t1;                                                 \
        (h) = t1 + t2;                                             \
    } while (0)

void sha256_transform(SHA256_CTX *ctx, const SHA256_BYTE data[]) {
    SHA256_WORD a, b, c, d, e, f, g, h;
    SHA256_WORD m[64];
    const SHA256_DWORD *data64 = (const SHA256_DWORD *)data;

#pragma GCC unroll 8
    for (int i = 0; i < 8; ++i) {
        SHA256_DWORD temp = __builtin_bswap64(data64[i]);
        m[2 * i] = (SHA256_WORD)(temp >> 32);
        m[2 * i + 1] = (SHA256_WORD)temp;
    }

    // Load state using 64-bit operations
    const SHA256_DWORD *state64 = (const SHA256_DWORD *)ctx->state;
    SHA256_DWORD s01 = state64[0];
    SHA256_DWORD s23 = state64[1];
    SHA256_DWORD s45 = state64[2];
    SHA256_DWORD s67 = state64[3];
    a = (SHA256_WORD)s01;
    b = (SHA256_WORD)(s01 >> 32);
    c = (SHA256_WORD)s23;
    d = (SHA256_WORD)(s23 >> 32);
    e = (SHA256_WORD)s45;
    f = (SHA256_WORD)(s45 >> 32);
    g = (SHA256_WORD)s67;
    h = (SHA256_WORD)(s67 >> 32);

#pragma GCC unroll 2
    for (int i = 0; i < 16; i += 8) {
        SHA256_ROUND(a, b, c, d, e, f, g, h, m[i], GLOBAL_K[i]);
        SHA256_ROUND(h, a, b, c, d, e, f, g, m[i + 1], GLOBAL_K[i + 1]);
        SHA256_ROUND(g, h, a, b, c, d, e, f, m[i + 2], GLOBAL_K[i + 2]);
        SHA256_ROUND(f, g, h, a, b, c, d, e, m[i + 3], GLOBAL_K[i + 3]);
        SHA256_ROUND(e, f, g, h, a, b, c, d, m[i + 4], GLOBAL_K[i + 4]);
        SHA256_ROUND(d, e, f, g, h, a, b, c, m[i + 5], GLOBAL_K[i + 5]);
        SHA256_ROUND(c, d, e, f, g, h, a, b, m[i + 6], GLOBAL_K[i + 6]);
        SHA256_ROUND(b, c, d, e, f, g, h, a, m[i + 7], GLOBAL_K[i + 7]);
    }

#pragma GCC unroll 6
    for (int i = 16; i < 64; i += 8) {
        m[i] = SIG1(m[i - 2]) + m[i - 7] + SIG0(m[i - 15]) + m[i - 16];
        SHA256_ROUND(a, b, c, d, e, f, g, h, m[i], GLOBAL_K[i]);
        m[i + 1] = SIG1(m[i - 1]) + m[i - 6] + SIG0(m[i - 14]) + m[i - 15];
        SHA256_ROUND(h, a, b, c, d, e, f, g, m[i + 1], GLOBAL_K[i + 1]);
        m[i + 2] = SIG1(m[i]) + m[i - 5] + SIG0(m[i - 13]) + m[i - 14];
        SHA256_ROUND(g, h, a, b, c, d, e, f, m[i + 2], GLOBAL_K[i + 2]);
        m[i + 3] = SIG1(m[i + 1]) + m[i - 4] + SIG0(m[i - 12]) + m[i - 13];
        SHA256_ROUND(f, g, h, a, b, c, d, e, m[i + 3], GLOBAL_K[i + 3]);
        m[i + 4] = SIG1(m[i + 2]) + m[i - 3] + SIG0(m[i - 11]) + m[i - 12];
        SHA256_ROUND(e, f, g, h, a, b, c, d, m[i + 4], GLOBAL_K[i + 4]);
        m[i + 5] = SIG1(m[i + 3]) + m[i - 2] + SIG0(m[i - 10]) + m[i - 11];
        SHA256_ROUND(d, e, f, g, h, a, b, c, m[i + 5], GLOBAL_K[i + 5]);
        m[i + 6] = SIG1(m[i + 4]) + m[i - 1] + SIG0(m[i - 9]) + m[i - 10];
        SHA256_ROUND(c, d, e, f, g, h, a, b, m[i + 6], GLOBAL_K[i + 6]);
        m[i + 7] = SIG1(m[i + 5]) + m[i] + SIG0(m[i - 8]) + m[i - 9];
        SHA256_ROUND(b, c, d, e, f, g, h, a, m[i + 7], GLOBAL_K[i + 7]);
    }

    ctx->state[0] += a;
    ctx->state[1] += b;
    ctx->state[2] += c;
    ctx->state[3] += d;
    ctx->state[4] += e;
    ctx->state[5] += f;
    ctx->state[6] += g;
    ctx->state[7] += h;
}

void sha256_init(SHA256_CTX *ctx) {
    ctx->datalen = 0;
    ctx->bitlen = 0;
    static const SHA256_DWORD initial_state[4] = {
        0xbb67ae856a09e667ULL, 0xa54ff53a3c6ef372ULL, 0x9b05688c510e527fULL,
        0x5be0cd191f83d9abULL};
    __builtin_memcpy(ctx->state, initial_state, sizeof(initial_state));
}

void sha256_update(SHA256_CTX *ctx, const SHA256_BYTE data[],
                   SHA256_DWORD len) {
    SHA256_WORD i = 0;

    while (i < len) {
        SHA256_DWORD space = 64 - ctx->datalen;
        SHA256_DWORD to_copy = (space < len - i) ? space : len - i;
        __builtin_memcpy(&ctx->data[ctx->datalen], &data[i], to_copy);
        ctx->datalen += to_copy;
        i += to_copy;
        if (ctx->datalen == 64) {
            sha256_transform(ctx, ctx->data);
            ctx->bitlen += 512;
            ctx->datalen = 0;
        }
    }
}

void sha256_final(SHA256_CTX *ctx, SHA256_BYTE hash[]) {
    SHA256_WORD i;

    i = ctx->datalen;

    // Pad whatever data is left in the buffer.
    if (ctx->datalen < 56) {
        ctx->data[i++] = 0x80;
        __builtin_memset(&ctx->data[i], 0, 56 - i);
    } else {
        ctx->data[i++] = 0x80;
        __builtin_memset(&ctx->data[i], 0, 64 - i);
        sha256_transform(ctx, ctx->data);
        __builtin_memset(ctx->data, 0, 56);
    }

    // Append to the padding the total message's length in bits and transform.
    ctx->bitlen += ctx->datalen * 8;
    SHA256_DWORD *data64 = (SHA256_DWORD *)&ctx->data[56];
    *data64 = __builtin_bswap64(ctx->bitlen);
    sha256_transform(ctx, ctx->data);

    const SHA256_DWORD *state64 = (const SHA256_DWORD *)ctx->state;
    SHA256_DWORD *hash64 = (SHA256_DWORD *)hash;
#pragma GCC unroll 4
    for (i = 0; i < 4; ++i) {
        // State stores [lo, hi] pairs, need to swap within 32-bit halves then
        // bswap64
        SHA256_DWORD s = state64[i];
        // Swap the two 32-bit halves: (lo, hi) -> (hi, lo)
        s = (s >> 32) | (s << 32);
        hash64[i] = __builtin_bswap64(s);
    }
}
