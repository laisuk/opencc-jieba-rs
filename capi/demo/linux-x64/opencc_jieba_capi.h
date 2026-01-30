#ifndef OPENCC_JIEBA_CAPI_H
#define OPENCC_JIEBA_CAPI_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>  // For size_t
#include <stdint.h>  // For standard integer types
#include <stdlib.h>  // For malloc/free
#include <stdbool.h> // For bool type

/**
 * Creates and initializes a new OpenCC JIEBA instance.
 *
 * This function allocates and returns a new instance used for conversion and segmentation.
 * The instance should be freed using `opencc_jieba_free()` when no longer needed.
 *
 * @return A pointer to a new instance of OpenCC JIEBA.
 */
void *opencc_jieba_new();

/**
 * Converts a null-terminated UTF-8 input string using the specified OpenCC config.
 *
 * @param instance     A pointer to the OpenCC instance created by `opencc_jieba_new()`.
 * @param input        The input UTF-8 string to convert.
 * @param config       The config name (e.g., "s2t", "t2s") for conversion rules.
 * @param punctuation  Whether to convert punctuation (true = convert).
 *
 * @return A newly allocated UTF-8 string with the converted output.
 *         The result must be freed using `opencc_jieba_free_string()`.
 */
char *opencc_jieba_convert(const void *instance, const char *input, const char *config, bool punctuation);

/**
 * Checks if the input string is valid Simplified or Traditional Chinese.
 *
 * @param instance A pointer to the OpenCC instance.
 * @param input    The input UTF-8 string to check.
 * @return An integer code indicating the check result:
 *         0 = Mixed/Undetermined,
 *         1 = Traditional Chinese,
 *         2 = Simplified Chinese,
 *         -1 = Invalid.
 */
int opencc_jieba_zho_check(const void *instance, const char *input);

/**
 * Frees an instance of OpenCC returned by `opencc_jieba_new`.
 *
 * @param instance A pointer to an instance previously returned by `opencc_jieba_new`.
 *                 Passing NULL is safe and does nothing.
 */
void opencc_jieba_delete(const void *instance);

/**
 * @deprecated Use `opencc_jieba_delete()` instead.
 *
 * Frees an instance of OpenCC returned by `opencc_jieba_new`.
 *
 * @param instance A pointer to an instance previously returned by `opencc_jieba_new`.
 *                 Passing NULL is safe and does nothing.
 */
void opencc_jieba_free(const void *instance);

/**
 * Frees a string returned by `opencc_jieba_convert`, `opencc_jieba_cut_and_join`,
 * `opencc_jieba_join_str`, or other string-returning functions.
 *
 * @param ptr A pointer to a string previously returned by the API.
 *            Passing NULL is safe and does nothing.
 */
void opencc_jieba_free_string(const char *ptr);

/**
 * Performs segmentation on a UTF-8 input string using Jieba.
 *
 * @param instance A pointer to the OpenCC instance.
 * @param input    The input UTF-8 string to segment.
 * @param hmm      Whether to enable the HMM model.
 *
 * @return A NULL-terminated array of UTF-8 C strings.
 *         Each element is a word segment.
 *         Must be freed using `opencc_jieba_free_string_array()`.
 */
char **opencc_jieba_cut(const void *instance, const char *input, bool hmm);

/**
 * Frees a NULL-terminated array of C strings returned by `opencc_jieba_cut`
 * or `opencc_jieba_keywords`.
 *
 * @param array A NULL-terminated array of strings to free.
 */
void opencc_jieba_free_string_array(char **array);

/**
 * Joins a NULL-terminated array of C strings into a single string using the given delimiter.
 *
 * @param strings   A NULL-terminated array of UTF-8 C strings.
 * @param delimiter A UTF-8 string delimiter to place between each string.
 *
 * @return A newly allocated string with all parts joined by the delimiter.
 *         Must be freed using `opencc_jieba_free_string()`.
 */
char *opencc_jieba_join_str(char **strings, const char *delimiter);

/**
 * Segments and joins an input string using Jieba, with the specified delimiter.
 *
 * @param instance  A pointer to the OpenCC instance.
 * @param input     A UTF-8 string to segment.
 * @param hmm       Whether to enable the HMM model.
 * @param delimiter A UTF-8 delimiter to insert between segments.
 *
 * @return A newly allocated string with segments joined by the delimiter.
 *         Must be freed using `opencc_jieba_free_string()`.
 */
char *opencc_jieba_cut_and_join(const void *instance, const char *input, bool hmm, const char *delimiter);

/**
 * Extracts keywords from input text using TF-IDF or TextRank.
 *
 * @param instance A pointer to the OpenCC instance.
 * @param input    A UTF-8 string to extract keywords from.
 * @param top_k    The number of top keywords to extract.
 * @param method   Extraction method: "tfidf" or "textrank".
 *
 * @return A NULL-terminated array of UTF-8 C strings representing keywords.
 *         Must be freed using `opencc_jieba_free_string_array()`.
 */
char **opencc_jieba_keywords(const void *instance, const char *input, int top_k, const char *method);

/**
 * Extracts keywords and their corresponding weights using TextRank or TF-IDF.
 *
 * @param instance     A pointer to the OpenCC instance.
 * @param input        The UTF-8 input string to analyze.
 * @param top_k        The number of top keywords to extract.
 * @param method       The extraction method ("tfidf" or "textrank").
 * @param out_len      Output pointer to store the number of extracted keywords.
 * @param out_keywords Output pointer to store the keyword strings array.
 * @param out_weights  Output pointer to store the keyword weights.
 *
 * @return 0 on success, negative value on error.
 *         `out_keywords` must be freed using `opencc_jieba_free_keywords_and_weights()`.
 */
int32_t opencc_jieba_keywords_and_weights(
    const void *instance,
    const char* input,
    size_t top_k,
    const char* method,
    size_t* out_len,
    char*** out_keywords,
    double** out_weights
);

/**
 * Frees memory allocated by `opencc_jieba_keywords_and_weights`.
 *
 * @param keywords Array of UTF-8 strings representing keywords.
 * @param weights  Array of weights associated with the keywords.
 * @param len      Number of keywords and weights.
 */
void opencc_jieba_free_keywords_and_weights(
    char** keywords,
    double* weights,
    size_t len
);

#ifdef __cplusplus
}
#endif

#endif // OPENCC_JIEBA_CAPI_H
