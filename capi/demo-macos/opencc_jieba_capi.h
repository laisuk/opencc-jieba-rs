#ifndef OPENCC_JIEBA_CAPI_H
#define OPENCC_JIEBA_CAPI_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>  // For size_t
#include <stdint.h>  // For standard integer types
#include <stdlib.h>  // For malloc/free
#include <stdbool.h> // For bool type

void *opencc_jieba_new();
char *opencc_jieba_convert(const void *instance, const char *input, const char *config, bool punctuation);
int opencc_jieba_zho_check(const void *instance, const char *input);
void opencc_jieba_free(const void *instance);
void opencc_jieba_free_string(const char *ptr);
char **opencc_jieba_cut(const void *instance, const char *input, bool hmm);
void opencc_jieba_free_string_array(char **array);
char *opencc_jieba_join_str(char **strings, const char *delimiter);
char *opencc_jieba_cut_and_join(const void *instance, const char *input, bool hmm, const char *delimiter);
char **opencc_jieba_keywords(const void *instance, const char *input, int top_k, const char *method);
// Function to extract keywords and their weights using TextRank/ TF-TIDF
// Parameters:
// - instance: Pointer to the OpenCC instance
// - input: Input text as a C string (null-terminated)
// - top_k: Number of top keywords to extract
// - method: TextRank/ TF-TIDF
// - out_len: Pointer to store the length of the returned keyword array
// - out_keywords: Pointer to store the array of extracted keywords (as C strings)
// - out_weights: Pointer to store the array of weights corresponding to the keywords
// Returns:
// - 0 on success, negative value on error
int32_t opencc_jieba_keywords_and_weights(
    const void *instance,
    const char* input,
    size_t top_k,
    const char* method,
    size_t* out_len,
    char*** out_keywords,
    double** out_weights
);

// Function to free the memory allocated for keywords and weights
// Parameters:
// - keywords: Pointer to the array of C strings containing keywords
// - weights: Pointer to the array of weights
// - len: Length of the arrays
void opencc_jieba_free_keywords_and_weights(
    char** keywords,
    double* weights,
    size_t len
);

#ifdef __cplusplus
}
#endif

#endif // OPENCC_JIEBA_CAPI_H
