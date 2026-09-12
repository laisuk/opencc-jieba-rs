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


typedef struct OpenccJiebaUserDictEntry {
    const char *word;
    size_t freq;
    const char *tag;
} OpenccJiebaUserDictEntry;

typedef uint32_t opencc_jieba_dict_slot_t;

enum {
    OPENCC_JIEBA_DICT_SLOT_ST_CHARACTERS = 1,
    OPENCC_JIEBA_DICT_SLOT_ST_PHRASES = 2,
    OPENCC_JIEBA_DICT_SLOT_TS_CHARACTERS = 3,
    OPENCC_JIEBA_DICT_SLOT_TS_PHRASES = 4,
    OPENCC_JIEBA_DICT_SLOT_TW_PHRASES = 5,
    OPENCC_JIEBA_DICT_SLOT_TW_PHRASES_REV = 6,
    OPENCC_JIEBA_DICT_SLOT_HK_PHRASES = 7,
    OPENCC_JIEBA_DICT_SLOT_HK_PHRASES_REV = 8,
    OPENCC_JIEBA_DICT_SLOT_TW_VARIANTS = 9,
    OPENCC_JIEBA_DICT_SLOT_TW_VARIANTS_PHRASES = 10,
    OPENCC_JIEBA_DICT_SLOT_TW_VARIANTS_REV = 11,
    OPENCC_JIEBA_DICT_SLOT_TW_VARIANTS_REV_PHRASES = 12,
    OPENCC_JIEBA_DICT_SLOT_HK_VARIANTS = 13,
    OPENCC_JIEBA_DICT_SLOT_HK_VARIANTS_PHRASES = 14,
    OPENCC_JIEBA_DICT_SLOT_HK_VARIANTS_REV = 15,
    OPENCC_JIEBA_DICT_SLOT_HK_VARIANTS_REV_PHRASES = 16,
    OPENCC_JIEBA_DICT_SLOT_JPS_CHARACTERS = 17,
    OPENCC_JIEBA_DICT_SLOT_JPS_CHARACTERS_REV = 18,
    OPENCC_JIEBA_DICT_SLOT_JPS_PHRASES = 19
};

typedef uint32_t opencc_jieba_custom_dict_mode_t;

enum {
    OPENCC_JIEBA_CUSTOM_DICT_APPEND = 1,
    OPENCC_JIEBA_CUSTOM_DICT_OVERRIDE = 2
};

typedef struct OpenccJiebaCustomPair {
    const char *source;
    const char *target;
} OpenccJiebaCustomPair;

typedef struct OpenccJiebaCustomDictSpec {
    opencc_jieba_dict_slot_t slot;
    opencc_jieba_custom_dict_mode_t mode;
    const OpenccJiebaCustomPair *pairs;
    size_t pair_count;
} OpenccJiebaCustomDictSpec;

/**
 * @typedef opencc_jieba_detofu_level_t
 *
 * @brief ABI-stable DeTofu threshold level.
 *
 * This type is a 32-bit unsigned integer. Level values are stable ABI
 * identifiers and will not be reordered or reused.
 *
 * DeTofu levels are threshold-based rather than dictionary-slot IDs, so the
 * first valid level intentionally starts at zero.
 *
 * @since Available since v0.8.1.
 */
typedef uint32_t opencc_jieba_detofu_level_t;

/**
 * @brief DeTofu fallback threshold values.
 *
 * The selected level is inclusive: the selected CJK extension and every
 * supported later extension are eligible for fallback replacement.
 *
 * `OPENCC_JIEBA_DETOFU_EXT_B` is the broadest level and covers all built-in
 * mappings from Extension B through Extension I. `OPENCC_JIEBA_DETOFU_EXT_I`
 * is the narrowest level and enables Extension I mappings only.
 *
 * @since Available since v0.8.1.
 */
enum {
    /** Replace Extension B and all supported later extension mappings. */
    OPENCC_JIEBA_DETOFU_EXT_B = 0,

    /** Replace Extension C and all supported later extension mappings. */
    OPENCC_JIEBA_DETOFU_EXT_C = 1,

    /** Replace Extension D and all supported later extension mappings. */
    OPENCC_JIEBA_DETOFU_EXT_D = 2,

    /** Replace Extension E and all supported later extension mappings. */
    OPENCC_JIEBA_DETOFU_EXT_E = 3,

    /** Replace Extension F and all supported later extension mappings. */
    OPENCC_JIEBA_DETOFU_EXT_F = 4,

    /** Replace Extension G and all supported later extension mappings. */
    OPENCC_JIEBA_DETOFU_EXT_G = 5,

    /** Replace Extension H and all supported later extension mappings. */
    OPENCC_JIEBA_DETOFU_EXT_H = 6,

    /** Replace Extension I mappings only. */
    OPENCC_JIEBA_DETOFU_EXT_I = 7
};

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

void *opencc_jieba_new_user_dict(
    const OpenccJiebaUserDictEntry *entries,
    size_t entry_count
);

void *opencc_jieba_new_custom(
    const OpenccJiebaCustomDictSpec *specs,
    size_t spec_count
);

void *opencc_jieba_new_user_dict_custom(
    const OpenccJiebaUserDictEntry *entries,
    size_t entry_count,
    const OpenccJiebaCustomDictSpec *specs,
    size_t spec_count
);

/**
 * @brief Destroys an OpenCC-Jieba instance.
 *
 * Passing NULL is safe and has no effect.
 *
 * @param instance Instance previously returned by any `opencc_jieba_new*()` constructor.
 */
void opencc_jieba_delete(void *instance);

/**
 * @brief Deprecated alias of `opencc_jieba_delete()`.
 *
 * Passing NULL is safe and has no effect.
 *
 * @param instance Instance previously returned by any `opencc_jieba_new*()` constructor.
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
 * @param config       Conversion config name. Supported values are `"s2t"`, `"s2tw"`,
 *                     `"s2twp"`, `"s2hk"`, `"s2hkp"`, `"t2s"`, `"t2tw"`, `"t2twp"`,
 *                     `"t2hk"`, `"t2hkp"`, `"tw2s"`, `"tw2sp"`, `"tw2t"`, `"tw2tp"`,
 *                     `"hk2s"`, `"hk2sp"`, `"hk2t"`, `"hk2tp"`, `"jp2t"`, or `"t2jp"`.
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
 * Compatibility normalization
 * ========================================================================= */

/**
 * @brief Normalizes CJK Compatibility Ideographs in a UTF-8 string.
 *
 * This is the C API counterpart of `OpenCC::normalize_compat()`.
 *
 * The operation replaces Unicode CJK Compatibility Ideographs with their
 * canonical unified ideograph forms using the built-in normalization table.
 * It is direction-independent and does not perform Simplified/Traditional
 * conversion, Jieba segmentation, punctuation conversion, or dictionary
 * mutation.
 *
 * This function is typically used as a preprocessing step before
 * `opencc_jieba_convert()` when input may contain CJK Compatibility
 * Ideographs, for example text extracted from PDFs or legacy documents.
 *
 * @param instance
 *     An OpenCC-Jieba instance returned by any `opencc_jieba_new*()`
 *     constructor.
 * @param input
 *     Input null-terminated UTF-8 string.
 *
 * @return
 *     A newly allocated null-terminated UTF-8 string on success.
 *
 *     Returns NULL if `instance` or `input` is NULL, or if `input` is not
 *     valid UTF-8. Retrieve the error immediately on the same calling thread
 *     using `opencc_jieba_last_error()`.
 *
 * @ownership
 *     The returned string is owned by the caller and must be released with
 *     `opencc_jieba_free_string()`.
 *
 * @since Available since v0.8.1.
 */
char *opencc_jieba_normalize_compat(
    const void *instance,
    const char *input
);

/**
 * @brief Applies the full built-in compatibility normalization pre-pass.
 *
 * This is the C API counterpart of `OpenCC::normalize_compat_extended()`.
 * It combines CJK Compatibility Ideograph normalization with the curated
 * Unicode compatibility mappings built into opencc-jieba-rs.
 *
 * In addition to CJK Compatibility Ideographs, the extended pass normalizes
 * selected Unicode radicals, glyph variants, punctuation forms, and known
 * text-extraction artifacts covered by the library's curated compatibility
 * table. Unmapped characters are preserved unchanged.
 *
 * The operation is independent of OpenCC conversion and does not modify the
 * instance, selected conversion config, dictionaries, Jieba segmentation,
 * script detection, IDS handling, or punctuation-conversion setting.
 *
 * For applications that want the complete compatibility preprocessing path,
 * this function is generally preferred over `opencc_jieba_normalize_compat()`.
 * Apply it before `opencc_jieba_convert()`; DeTofu, when desired, is normally
 * applied after conversion.
 *
 * @param instance
 *     An OpenCC-Jieba instance returned by any `opencc_jieba_new*()`
 *     constructor.
 * @param input
 *     Input null-terminated UTF-8 string.
 *
 * @return
 *     A newly allocated null-terminated UTF-8 string on success.
 *
 *     Returns NULL if `instance` or `input` is NULL, or if `input` is not
 *     valid UTF-8. Retrieve the error immediately on the same calling thread
 *     using `opencc_jieba_last_error()`.
 *
 * @ownership
 *     The returned string is owned by the caller and must be released with
 *     `opencc_jieba_free_string()`.
 *
 * @since Available since v0.8.1.
 */
char *opencc_jieba_normalize_compat_extended(
    const void *instance,
    const char *input
);

/* =========================================================================
 * DeTofu
 * ========================================================================= */

/**
 * @brief Applies the built-in DeTofu display-compatibility fallback.
 *
 * DeTofu replaces selected rare non-BMP CJK extension characters with
 * display-safer fallback characters from the built-in table. It is intended
 * for environments where rare extension characters may render as tofu boxes,
 * missing-glyph placeholders, or otherwise unsupported glyphs.
 *
 * DeTofu is direction-independent and does not modify OpenCC conversion
 * dictionaries, Jieba segmentation, phrase matching, regional variants, or
 * punctuation conversion. In a normal conversion pipeline, apply it after
 * `opencc_jieba_convert()`.
 *
 * The threshold is inclusive. For example,
 * `OPENCC_JIEBA_DETOFU_EXT_B` enables all supported built-in mappings from
 * Extension B through Extension I, while `OPENCC_JIEBA_DETOFU_EXT_I` enables
 * Extension I mappings only.
 *
 * @param instance
 *     An OpenCC-Jieba instance returned by any `opencc_jieba_new*()`
 *     constructor.
 * @param input
 *     Input null-terminated UTF-8 string.
 * @param level
 *     DeTofu threshold such as `OPENCC_JIEBA_DETOFU_EXT_B`.
 *
 * @return
 *     A newly allocated null-terminated UTF-8 string on success.
 *
 *     Returns NULL if `instance` or `input` is NULL, if `input` is not valid
 *     UTF-8, or if `level` is not a recognized
 *     `opencc_jieba_detofu_level_t` value. Retrieve the error immediately on
 *     the same calling thread with `opencc_jieba_last_error()`.
 *
 * @ownership
 *     The returned string is owned by the caller and must be released with
 *     `opencc_jieba_free_string()`.
 *
 * @since Available since v0.8.1.
 */
char *opencc_jieba_detofu(
    const void *instance,
    const char *input,
    opencc_jieba_detofu_level_t level
);

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
 * Error state
 * ========================================================================= */

/**
 * @brief Returns the calling thread's last C API error.
 *
 * The returned string is newly allocated and must be released with
 * `opencc_jieba_free_string()`.
 */
char *opencc_jieba_last_error(void);

/** @brief Clears the calling thread's last C API error. */
void opencc_jieba_clear_last_error(void);

/* =========================================================================
 * Memory management
 * ========================================================================= */

/**
 * @brief Frees a string returned by this API.
 *
 * This includes strings returned by `opencc_jieba_convert()`,
 * `opencc_jieba_normalize_compat()`,
 * `opencc_jieba_normalize_compat_extended()`, `opencc_jieba_detofu()`,
 * `opencc_jieba_cut_and_join()`,
 * `opencc_jieba_join_str()`, and `opencc_jieba_last_error()`.
 *
 * Passing NULL is safe and has no effect.
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