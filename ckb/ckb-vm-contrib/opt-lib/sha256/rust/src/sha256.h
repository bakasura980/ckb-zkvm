/*********************************************************************
 * Filename:   sha256.h
 * Author:     Brad Conte (brad AT bradconte.com)
 * Copyright:
 * Disclaimer: This code is presented "as is" without any guarantees.
 * Details:    Defines the API for the corresponding SHA1 implementation.
 ********************************************************************/

#ifndef SHA256_H
#define SHA256_H

/****************************** MACROS ******************************/
#define SHA256_BLOCK_SIZE 32  // SHA256 outputs a 32 byte digest

/**************************** DATA TYPES ****************************/
// 8-bit byte
typedef unsigned char SHA256_BYTE;
// 32-bit word, change to "long" for 16-bit machines
typedef unsigned int SHA256_WORD;
// 64-bit dword
typedef unsigned long long SHA256_DWORD;

typedef struct {
    SHA256_BYTE data[64];
    SHA256_WORD datalen;
    SHA256_DWORD bitlen;
    SHA256_WORD state[8];
} SHA256_CTX;

/*********************** FUNCTION DECLARATIONS **********************/
void sha256_init(SHA256_CTX *ctx);
void sha256_update(SHA256_CTX *ctx, const SHA256_BYTE data[], SHA256_DWORD len);
void sha256_final(SHA256_CTX *ctx, SHA256_BYTE hash[]);
#endif  // SHA256_H
