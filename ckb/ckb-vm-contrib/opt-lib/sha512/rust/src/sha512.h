/*********************************************************************
 * Filename:   sha512.h
 * Author:     Brad Conte (brad AT bradconte.com)
 * Copyright:
 * Disclaimer: This code is presented "as is" without any guarantees.
 * Details:    Defines the API for the corresponding SHA1 implementation.
 ********************************************************************/

#ifndef SHA512_H
#define SHA512_H

/****************************** MACROS ******************************/
#define SHA512_BLOCK_SIZE 64  // SHA512 outputs a 64 byte digest

/**************************** DATA TYPES ****************************/
// 8-bit byte
typedef unsigned char SHA512_BYTE;
// 64-bit word
typedef unsigned long long SHA512_WORD;
// 128-bit dword
typedef unsigned __int128 SHA512_DWORD;

typedef struct {
    SHA512_BYTE data[128];
    SHA512_WORD datalen;
    SHA512_DWORD bitlen;
    SHA512_WORD state[8];
} SHA512_CTX;

/*********************** FUNCTION DECLARATIONS **********************/
void sha512_init(SHA512_CTX *ctx);
void sha512_update(SHA512_CTX *ctx, const SHA512_BYTE data[], SHA512_WORD len);
void sha512_final(SHA512_CTX *ctx, SHA512_BYTE hash[]);
#endif  // SHA512_H
