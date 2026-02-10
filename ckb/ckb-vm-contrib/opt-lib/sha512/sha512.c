/*************************** HEADER FILES ***************************/
#include "sha512.h"

/****************************** MACROS ******************************/
#define ROTRIGHT(a, b) (((a) >> (b)) | ((a) << (64 - (b))))

#define CH(x, y, z) (((x) & (y)) ^ (~(x) & (z)))
#define MAJ(x, y, z) ((((x) | (y)) & (z)) | ((x) & (y)))
#define EP0(x) (ROTRIGHT(x, 28) ^ ROTRIGHT(x, 34) ^ ROTRIGHT(x, 39))
#define EP1(x) (ROTRIGHT(x, 14) ^ ROTRIGHT(x, 18) ^ ROTRIGHT(x, 41))
#define SIG0(x) (ROTRIGHT(x, 1) ^ ROTRIGHT(x, 8) ^ ((x) >> 7))
#define SIG1(x) (ROTRIGHT(x, 19) ^ ROTRIGHT(x, 61) ^ ((x) >> 6))

/**************************** VARIABLES *****************************/
static const SHA512_WORD GLOBAL_K[80] = {
    0x428a2f98d728ae22ULL, 0x7137449123ef65cdULL, 0xb5c0fbcfec4d3b2fULL,
    0xe9b5dba58189dbbcULL, 0x3956c25bf348b538ULL, 0x59f111f1b605d019ULL,
    0x923f82a4af194f9bULL, 0xab1c5ed5da6d8118ULL, 0xd807aa98a3030242ULL,
    0x12835b0145706fbeULL, 0x243185be4ee4b28cULL, 0x550c7dc3d5ffb4e2ULL,
    0x72be5d74f27b896fULL, 0x80deb1fe3b1696b1ULL, 0x9bdc06a725c71235ULL,
    0xc19bf174cf692694ULL, 0xe49b69c19ef14ad2ULL, 0xefbe4786384f25e3ULL,
    0x0fc19dc68b8cd5b5ULL, 0x240ca1cc77ac9c65ULL, 0x2de92c6f592b0275ULL,
    0x4a7484aa6ea6e483ULL, 0x5cb0a9dcbd41fbd4ULL, 0x76f988da831153b5ULL,
    0x983e5152ee66dfabULL, 0xa831c66d2db43210ULL, 0xb00327c898fb213fULL,
    0xbf597fc7beef0ee4ULL, 0xc6e00bf33da88fc2ULL, 0xd5a79147930aa725ULL,
    0x06ca6351e003826fULL, 0x142929670a0e6e70ULL, 0x27b70a8546d22ffcULL,
    0x2e1b21385c26c926ULL, 0x4d2c6dfc5ac42aedULL, 0x53380d139d95b3dfULL,
    0x650a73548baf63deULL, 0x766a0abb3c77b2a8ULL, 0x81c2c92e47edaee6ULL,
    0x92722c851482353bULL, 0xa2bfe8a14cf10364ULL, 0xa81a664bbc423001ULL,
    0xc24b8b70d0f89791ULL, 0xc76c51a30654be30ULL, 0xd192e819d6ef5218ULL,
    0xd69906245565a910ULL, 0xf40e35855771202aULL, 0x106aa07032bbd1b8ULL,
    0x19a4c116b8d2d0c8ULL, 0x1e376c085141ab53ULL, 0x2748774cdf8eeb99ULL,
    0x34b0bcb5e19b48a8ULL, 0x391c0cb3c5c95a63ULL, 0x4ed8aa4ae3418acbULL,
    0x5b9cca4f7763e373ULL, 0x682e6ff3d6b2b8a3ULL, 0x748f82ee5defb2fcULL,
    0x78a5636f43172f60ULL, 0x84c87814a1f0ab72ULL, 0x8cc702081a6439ecULL,
    0x90befffa23631e28ULL, 0xa4506cebde82bde9ULL, 0xbef9a3f7b2c67915ULL,
    0xc67178f2e372532bULL, 0xca273eceea26619cULL, 0xd186b8c721c0c207ULL,
    0xeada7dd6cde0eb1eULL, 0xf57d4f7fee6ed178ULL, 0x06f067aa72176fbaULL,
    0x0a637dc5a2c898a6ULL, 0x113f9804bef90daeULL, 0x1b710b35131c471bULL,
    0x28db77f523047d84ULL, 0x32caab7b40c72493ULL, 0x3c9ebe0a15c9bebcULL,
    0x431d67c49c100d4cULL, 0x4cc5d4becb3e42b6ULL, 0x597f299cfc657e2aULL,
    0x5fcb6fab3ad6faecULL, 0x6c44198c4a475817ULL};

/*********************** FUNCTION DEFINITIONS ***********************/
#define SHA512_ROUND(a, b, c, d, e, f, g, h, ki, mi)               \
    do {                                                           \
        SHA512_WORD t1 = (h) + EP1(e) + CH(e, f, g) + (ki) + (mi); \
        SHA512_WORD t2 = EP0(a) + MAJ(a, b, c);                    \
        (d) += t1;                                                 \
        (h) = t1 + t2;                                             \
    } while (0)

void sha512_transform(SHA512_CTX *ctx, const SHA512_BYTE data[]) {
    SHA512_WORD a, b, c, d, e, f, g, h;
    SHA512_WORD m[80];
    const SHA512_WORD *data64 = (const SHA512_WORD *)data;

#pragma GCC unroll 16
    for (int i = 0; i < 16; ++i) {
        m[i] = __builtin_bswap64(data64[i]);
    }

    a = ctx->state[0];
    b = ctx->state[1];
    c = ctx->state[2];
    d = ctx->state[3];
    e = ctx->state[4];
    f = ctx->state[5];
    g = ctx->state[6];
    h = ctx->state[7];

#pragma GCC unroll 2
    for (int i = 0; i < 16; i += 8) {
        SHA512_ROUND(a, b, c, d, e, f, g, h, m[i], GLOBAL_K[i]);
        SHA512_ROUND(h, a, b, c, d, e, f, g, m[i + 1], GLOBAL_K[i + 1]);
        SHA512_ROUND(g, h, a, b, c, d, e, f, m[i + 2], GLOBAL_K[i + 2]);
        SHA512_ROUND(f, g, h, a, b, c, d, e, m[i + 3], GLOBAL_K[i + 3]);
        SHA512_ROUND(e, f, g, h, a, b, c, d, m[i + 4], GLOBAL_K[i + 4]);
        SHA512_ROUND(d, e, f, g, h, a, b, c, m[i + 5], GLOBAL_K[i + 5]);
        SHA512_ROUND(c, d, e, f, g, h, a, b, m[i + 6], GLOBAL_K[i + 6]);
        SHA512_ROUND(b, c, d, e, f, g, h, a, m[i + 7], GLOBAL_K[i + 7]);
    }

#pragma GCC unroll 8
    for (int i = 16; i < 80; i += 8) {
        m[i] = SIG1(m[i - 2]) + m[i - 7] + SIG0(m[i - 15]) + m[i - 16];
        SHA512_ROUND(a, b, c, d, e, f, g, h, m[i], GLOBAL_K[i]);
        m[i + 1] = SIG1(m[i - 1]) + m[i - 6] + SIG0(m[i - 14]) + m[i - 15];
        SHA512_ROUND(h, a, b, c, d, e, f, g, m[i + 1], GLOBAL_K[i + 1]);
        m[i + 2] = SIG1(m[i]) + m[i - 5] + SIG0(m[i - 13]) + m[i - 14];
        SHA512_ROUND(g, h, a, b, c, d, e, f, m[i + 2], GLOBAL_K[i + 2]);
        m[i + 3] = SIG1(m[i + 1]) + m[i - 4] + SIG0(m[i - 12]) + m[i - 13];
        SHA512_ROUND(f, g, h, a, b, c, d, e, m[i + 3], GLOBAL_K[i + 3]);
        m[i + 4] = SIG1(m[i + 2]) + m[i - 3] + SIG0(m[i - 11]) + m[i - 12];
        SHA512_ROUND(e, f, g, h, a, b, c, d, m[i + 4], GLOBAL_K[i + 4]);
        m[i + 5] = SIG1(m[i + 3]) + m[i - 2] + SIG0(m[i - 10]) + m[i - 11];
        SHA512_ROUND(d, e, f, g, h, a, b, c, m[i + 5], GLOBAL_K[i + 5]);
        m[i + 6] = SIG1(m[i + 4]) + m[i - 1] + SIG0(m[i - 9]) + m[i - 10];
        SHA512_ROUND(c, d, e, f, g, h, a, b, m[i + 6], GLOBAL_K[i + 6]);
        m[i + 7] = SIG1(m[i + 5]) + m[i] + SIG0(m[i - 8]) + m[i - 9];
        SHA512_ROUND(b, c, d, e, f, g, h, a, m[i + 7], GLOBAL_K[i + 7]);
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

void sha512_init(SHA512_CTX *ctx) {
    ctx->datalen = 0;
    ctx->bitlen = 0;
    static const SHA512_WORD initial_state[8] = {
        0x6a09e667f3bcc908ULL, 0xbb67ae8584caa73bULL, 0x3c6ef372fe94f82bULL,
        0xa54ff53a5f1d36f1ULL, 0x510e527fade682d1ULL, 0x9b05688c2b3e6c1fULL,
        0x1f83d9abfb41bd6bULL, 0x5be0cd19137e2179ULL};
    __builtin_memcpy(ctx->state, initial_state, sizeof(initial_state));
}

void sha512_update(SHA512_CTX *ctx, const SHA512_BYTE data[], SHA512_WORD len) {
    SHA512_WORD i = 0;

    while (i < len) {
        SHA512_WORD space = 128 - ctx->datalen;
        SHA512_WORD to_copy = (space < len - i) ? space : len - i;
        __builtin_memcpy(&ctx->data[ctx->datalen], &data[i], to_copy);
        ctx->datalen += to_copy;
        i += to_copy;
        if (ctx->datalen == 128) {
            sha512_transform(ctx, ctx->data);
            ctx->bitlen += 1024;
            ctx->datalen = 0;
        }
    }
}

void sha512_final(SHA512_CTX *ctx, SHA512_BYTE hash[]) {
    SHA512_WORD i;

    i = ctx->datalen;

    // Pad whatever data is left in the buffer.
    if (ctx->datalen < 112) {
        ctx->data[i++] = 0x80;
        __builtin_memset(&ctx->data[i], 0, 112 - i);
    } else {
        ctx->data[i++] = 0x80;
        __builtin_memset(&ctx->data[i], 0, 128 - i);
        sha512_transform(ctx, ctx->data);
        __builtin_memset(ctx->data, 0, 112);
    }

    // Append to the padding the total message's length in bits and transform.
    ctx->bitlen += ctx->datalen * 8;
    SHA512_DWORD *data128 = (SHA512_DWORD *)&ctx->data[112];
    SHA512_DWORD bitlen_be =
        ((SHA512_DWORD)__builtin_bswap64((SHA512_WORD)ctx->bitlen) << 64) |
        (SHA512_DWORD)__builtin_bswap64((SHA512_WORD)(ctx->bitlen >> 64));
    *data128 = bitlen_be;
    sha512_transform(ctx, ctx->data);

    SHA512_WORD *hash64 = (SHA512_WORD *)hash;
#pragma GCC unroll 8
    for (i = 0; i < 8; ++i) {
        hash64[i] = __builtin_bswap64(ctx->state[i]);
    }
}
