#ifndef OPENCC_JIEBA_CAPI_H
#define OPENCC_JIEBA_CAPI_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief A Jieba token and its corresponding part-of-speech tag.
 *
 * Both fields are UTF-8, null-terminated strings.
 *
 * Arrays returned by this API are terminated by a sentinel entry where both
 * `word` and `tag` are NULL.
 */
typedef struct OpenccJiebaTag {
    char *word;
    char *tag;
} OpenccJiebaTag;

/* =========================================================================
 * Metadata
 * ========================================================================= */

/**
 * @brief Returns the OpenCC-Jieba C ABI version number.
 *
 * This value is intended for runtime compatibility checks and only changes
 * when the C ABI is broken.
 *
 * @return ABI version number.
 */
uint32_t opencc_jieba_abi_number(void);

/**
 * @brief Returns the OpenCC-Jieba version string.
 *
 * The returned pointer is a UTF-8, null-terminated string valid for the
 * lifetime of the program. It must not be freed.
 *
 * Example: `"0.7.4-beta1"`
 *
 * @return Version string pointer.
 */
const char *opencc_jieba_version_string(void);

/* =========================================================================
 * Lifecycle
 * ========================================================================= */

/**
 * @brief Creates a new OpenCC-Jieba instance.
 *
 * The returned instance is used for conversion, segmentation, tagging,
 * and keyword extraction.
 *
 * Destroy it with `opencc_jieba_delete()` when no longer needed.
 *
 * @return Pointer to a newly allocated instance, or NULL on failure.
 */
void *opencc_jieba_new(void);

/**
 * @brief Destroys an OpenCC-Jieba instance.
 *
 * Passing NULL is safe and has no effect.
 *
 * @param instance Instance previously returned by `opencc_jieba_new()`.
 */
void opencc_jieba_delete(void *instance);

/**
 * @brief Deprecated alias of `opencc_jieba_delete()`.
 *
 * Passing NULL is safe and has no effect.
 *
 * @param instance Instance previously returned by `opencc_jieba_new()`.
 */
void opencc_jieba_free(void *instance);

/* =========================================================================
 * Conversion
 * ========================================================================= */

/**
 * @brief Converts text using the specified OpenCC configuration.
 *
 * @param instance     Instance created by `opencc_jieba_new()`.
 * @param input        Input UTF-8, null-terminated string.
 * @param config       Conversion config name, such as `"s2t"` or `"t2s"`.
 * @param punctuation  Whether punctuation conversion is enabled.
 *
 * @return Newly allocated UTF-8 string. Free with `opencc_jieba_free_string()`.
 *         Returns NULL on error.
 */
char *opencc_jieba_convert(
    const void *instance,
    const char *input,
    const char *config,
    bool punctuation
);

/**
 * @brief Checks whether input text is Simplified or Traditional Chinese.
 *
 * Return values:
 * - `0`  = mixed / undetermined
 * - `1`  = Traditional Chinese
 * - `2`  = Simplified Chinese
 * - `-1` = invalid input or error
 *
 * @param instance Instance created by `opencc_jieba_new()`.
 * @param input    Input UTF-8, null-terminated string.
 *
 * @return Status code described above.
 */
int opencc_jieba_zho_check(const void *instance, const char *input);

/* =========================================================================
 * Segmentation and tagging
 * ========================================================================= */

/**
 * @brief Segments text using Jieba default cut mode.
 *
 * @param instance Instance created by `opencc_jieba_new()`.
 * @param input    Input UTF-8, null-terminated string.
 * @param hmm      Whether to enable HMM new-word discovery.
 *
 * @return NULL-terminated array of UTF-8 strings.
 *         Free with `opencc_jieba_free_string_array()`.
 *         Returns NULL on error.
 */
char **opencc_jieba_cut(const void *instance, const char *input, bool hmm);

/**
 * @brief Segments text using Jieba full mode.
 *
 * @param instance Instance created by `opencc_jieba_new()`.
 * @param input    Input UTF-8, null-terminated string.
 *
 * @return NULL-terminated array of UTF-8 strings.
 *         Free with `opencc_jieba_free_string_array()`.
 *         Returns NULL on error.
 */
char **opencc_jieba_cut_all(const void *instance, const char *input);

/**
 * @brief Segments text using Jieba search mode.
 *
 * @param instance Instance created by `opencc_jieba_new()`.
 * @param input    Input UTF-8, null-terminated string.
 * @param hmm      Whether to enable HMM new-word discovery.
 *
 * @return NULL-terminated array of UTF-8 strings.
 *         Free with `opencc_jieba_free_string_array()`.
 *         Returns NULL on error.
 */
char **opencc_jieba_cut_for_search(const void *instance, const char *input, bool hmm);

/**
 * @brief Segments text and joins the tokens with a delimiter.
 *
 * @param instance  Instance created by `opencc_jieba_new()`.
 * @param input     Input UTF-8, null-terminated string.
 * @param hmm       Whether to enable HMM new-word discovery.
 * @param delimiter UTF-8 delimiter inserted between tokens.
 *
 * @return Newly allocated UTF-8 string.
 *         Free with `opencc_jieba_free_string()`.
 *         Returns NULL on error.
 */
char *opencc_jieba_cut_and_join(
    const void *instance,
    const char *input,
    bool hmm,
    const char *delimiter
);

/**
 * @brief Performs Jieba part-of-speech tagging.
 *
 * @param instance Instance created by `opencc_jieba_new()`.
 * @param input    Input UTF-8, null-terminated string.
 * @param hmm      Whether to enable HMM new-word discovery.
 *
 * @return Sentinel-terminated array of `OpenccJiebaTag`.
 *         Free with `opencc_jieba_free_tag_array()`.
 *         Returns NULL on error.
 */
OpenccJiebaTag *opencc_jieba_tag(const void *instance, const char *input, bool hmm);

/**
 * @brief Joins a NULL-terminated string array using a delimiter.
 *
 * @param strings    NULL-terminated array of UTF-8 strings.
 * @param delimiter  UTF-8 delimiter inserted between strings.
 *
 * @return Newly allocated UTF-8 string.
 *         Free with `opencc_jieba_free_string()`.
 *         Returns NULL on error.
 */
char *opencc_jieba_join_str(const char *const *strings, const char *delimiter);

/* =========================================================================
 * Keyword extraction
 * ========================================================================= */

/**
 * @brief Extracts top keywords using TextRank or TF-IDF.
 *
 * `method` must be `"textrank"` or `"tfidf"`.
 *
 * @param instance Instance created by `opencc_jieba_new()`.
 * @param input    Input UTF-8, null-terminated string.
 * @param top_k    Maximum number of keywords to return.
 * @param method   Keyword extraction method: `"textrank"` or `"tfidf"`.
 *
 * @return NULL-terminated array of UTF-8 keyword strings.
 *         Free with `opencc_jieba_free_string_array()`.
 *         Returns NULL on error.
 */
char **opencc_jieba_keywords(
    const void *instance,
    const char *input,
    size_t top_k,
    const char *method
);

/**
 * @brief Extracts top keywords using TextRank or TF-IDF with optional POS filtering.
 *
 * `method` must be `"textrank"` or `"tfidf"`.
 *
 * `allowed_pos` is a UTF-8, null-terminated, space-separated POS list such as:
 * `"n nr ns nt nz v vn"`.
 *
 * If `allowed_pos` is NULL or an empty string, no POS filtering is applied.
 *
 * @param instance     Instance created by `opencc_jieba_new()`.
 * @param input        Input UTF-8, null-terminated string.
 * @param top_k        Maximum number of keywords to return.
 * @param method       Keyword extraction method: `"textrank"` or `"tfidf"`.
 * @param allowed_pos  Optional space-separated POS filter list.
 *
 * @return NULL-terminated array of UTF-8 keyword strings.
 *         Free with `opencc_jieba_free_string_array()`.
 *         Returns NULL on error.
 */
char **opencc_jieba_keywords_pos(
    const void *instance,
    const char *input,
    size_t top_k,
    const char *method,
    const char *allowed_pos
);

/**
 * @brief Extracts keywords and their weights using TextRank or TF-IDF.
 *
 * `method` must be `"textrank"` or `"tfidf"`.
 *
 * On success:
 * - `*out_len` receives the number of keywords
 * - `*out_keywords` receives an array of UTF-8 strings
 * - `*out_weights` receives an array of `double`
 *
 * Free both output arrays with `opencc_jieba_free_keywords_and_weights()`.
 *
 * @param instance      Instance created by `opencc_jieba_new()`.
 * @param input         Input UTF-8, null-terminated string.
 * @param top_k         Maximum number of keywords to return.
 * @param method        Keyword extraction method: `"textrank"` or `"tfidf"`.
 * @param out_len       Output keyword count.
 * @param out_keywords  Output keyword array.
 * @param out_weights   Output weight array.
 *
 * @return `0` on success, negative value on error.
 */
int32_t opencc_jieba_keywords_and_weights(
    const void *instance,
    const char *input,
    size_t top_k,
    const char *method,
    size_t *out_len,
    char ***out_keywords,
    double **out_weights
);

/**
 * @brief Extracts keywords and their weights using TextRank or TF-IDF
 *        with optional POS filtering.
 *
 * `method` must be `"textrank"` or `"tfidf"`.
 *
 * `allowed_pos` is a UTF-8, null-terminated, space-separated POS list such as:
 * `"n nr ns nt nz v vn"`.
 *
 * If `allowed_pos` is NULL or an empty string, no POS filtering is applied.
 *
 * On success:
 * - `*out_len` receives the number of keywords
 * - `*out_keywords` receives an array of UTF-8 strings
 * - `*out_weights` receives an array of `double`
 *
 * Free both output arrays with `opencc_jieba_free_keywords_and_weights()`.
 *
 * @param instance      Instance created by `opencc_jieba_new()`.
 * @param input         Input UTF-8, null-terminated string.
 * @param top_k         Maximum number of keywords to return.
 * @param method        Keyword extraction method: `"textrank"` or `"tfidf"`.
 * @param allowed_pos   Optional space-separated POS filter list.
 * @param out_len       Output keyword count.
 * @param out_keywords  Output keyword array.
 * @param out_weights   Output weight array.
 *
 * @return `0` on success, negative value on error.
 */
int32_t opencc_jieba_keywords_and_weights_pos(
    const void *instance,
    const char *input,
    size_t top_k,
    const char *method,
    const char *allowed_pos,
    size_t *out_len,
    char ***out_keywords,
    double **out_weights
);

/* =========================================================================
 * Memory management
 * ========================================================================= */

/**
 * @brief Frees a string returned by this API.
 *
 * Safe to call with NULL.
 *
 * @param ptr String pointer previously returned by this API.
 */
void opencc_jieba_free_string(char *ptr);

/**
 * @brief Frees a NULL-terminated string array returned by this API.
 *
 * Safe to call with NULL.
 *
 * This is used for arrays returned by:
 * - `opencc_jieba_cut()`
 * - `opencc_jieba_cut_all()`
 * - `opencc_jieba_cut_for_search()`
 * - `opencc_jieba_keywords()`
 * - `opencc_jieba_keywords_pos()`
 *
 * @param array NULL-terminated array of UTF-8 strings.
 */
void opencc_jieba_free_string_array(char **array);

/**
 * @brief Frees a sentinel-terminated tag array returned by `opencc_jieba_tag()`.
 *
 * Safe to call with NULL.
 *
 * @param array Array terminated by an entry where both `word` and `tag` are NULL.
 */
void opencc_jieba_free_tag_array(OpenccJiebaTag *array);

/**
 * @brief Frees arrays returned by weighted keyword extraction functions.
 *
 * Safe to call with NULL pointers.
 *
 * This is used for outputs returned by:
 * - `opencc_jieba_keywords_and_weights()`
 * - `opencc_jieba_keywords_and_weights_pos()`
 *
 * @param keywords Array of UTF-8 keyword strings.
 * @param weights  Array of keyword weights.
 * @param len      Number of elements in both arrays.
 */
void opencc_jieba_free_keywords_and_weights(
    char **keywords,
    double *weights,
    size_t len
);

#ifdef __cplusplus
}
#endif

#endif /* OPENCC_JIEBA_CAPI_H */