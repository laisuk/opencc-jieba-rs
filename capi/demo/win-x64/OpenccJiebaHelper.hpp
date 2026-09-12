#pragma once

#include "opencc_jieba_capi.h"

#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

// RAII convenience wrapper around the opencc-jieba C API.
//
// This helper owns exactly one native OpenCC-Jieba instance and releases it
// with `opencc_jieba_delete()` in the destructor.
//
// It preserves the native API's behavior where practical:
// - Conversion config names are passed through unchanged.
// - Native failures that return NULL are surfaced through `lastError()`.
// - Returned native strings and arrays are copied into standard C++ types and
//   released immediately with the matching C API free function.
// - Custom Jieba user dictionaries and custom OpenCC conversion dictionaries
//   are copied by the native constructors, so caller-owned input storage only
//   needs to remain valid for the duration of construction.
class OpenccJiebaHelper {
public:
    /**
     * One owned Jieba user-dictionary entry used during construction.
     *
     * `word` and `tag`, when present, must be valid UTF-8 and must not contain
     * embedded NUL bytes.
     *
     * An empty `tag` is passed as NULL to the native API.
     */
    struct UserDictEntry {
        /** Word or phrase to add to the Jieba segmentation dictionary. */
        std::string word;

        /** Jieba frequency value. */
        std::size_t freq;

        /** Optional Jieba part-of-speech tag. Empty means no tag. */
        std::string tag;
    };

    /**
     * One owned UTF-8 source-target conversion mapping.
     */
    struct CustomPair {
        /** Source dictionary key. */
        std::string source;

        /** Replacement dictionary value. */
        std::string target;
    };

    /**
     * One owned custom OpenCC conversion-dictionary specification.
     *
     * `slot` must be one of the `OPENCC_JIEBA_DICT_SLOT_*` constants and
     * `mode` must be `OPENCC_JIEBA_CUSTOM_DICT_APPEND` or
     * `OPENCC_JIEBA_CUSTOM_DICT_OVERRIDE`.
     */
    struct CustomDictSpec {
        opencc_jieba_dict_slot_t slot;
        opencc_jieba_custom_dict_mode_t mode;
        std::vector<CustomPair> pairs;
    };

    /**
     * One Jieba tagged token.
     */
    struct Tag {
        std::string word;
        std::string tag;
    };

    /**
     * One keyword and its weight.
     */
    struct WeightedKeyword {
        std::string keyword;
        double weight;
    };

    // Creates a new native OpenCC-Jieba instance with the built-in Jieba and
    // OpenCC dictionaries.
    //
    // Throws std::runtime_error if native construction fails.
    OpenccJiebaHelper()
        : instance_(createDefault()) {}

    /**
     * Creates an OpenCC-Jieba instance with custom Jieba user-dictionary
     * entries and the built-in OpenCC conversion dictionaries.
     */
    explicit OpenccJiebaHelper(const std::vector<UserDictEntry> &entries)
        : instance_(createWithUserDict(entries)) {}

    /**
     * Creates an OpenCC-Jieba instance with the built-in Jieba dictionary and
     * custom OpenCC conversion dictionaries.
     */
    explicit OpenccJiebaHelper(const std::vector<CustomDictSpec> &specs)
        : instance_(createWithCustomDicts(specs)) {}

    /**
     * Creates an OpenCC-Jieba instance with both custom Jieba segmentation
     * entries and custom OpenCC conversion dictionaries.
     *
     * This is the recommended constructor when custom phrase mappings depend
     * on Jieba preserving the source phrase as a single token.
     */
    OpenccJiebaHelper(
        const std::vector<UserDictEntry> &entries,
        const std::vector<CustomDictSpec> &specs
    )
        : instance_(createWithUserDictAndCustom(entries, specs)) {}

    OpenccJiebaHelper(const OpenccJiebaHelper &) = delete;
    OpenccJiebaHelper &operator=(const OpenccJiebaHelper &) = delete;

    OpenccJiebaHelper(OpenccJiebaHelper &&other) noexcept
        : instance_(std::exchange(other.instance_, nullptr)),
          config_(std::move(other.config_)),
          punctuationEnabled_(other.punctuationEnabled_) {}

    OpenccJiebaHelper &operator=(OpenccJiebaHelper &&other) noexcept {
        if (this != &other) {
            cleanup();
            instance_ = std::exchange(other.instance_, nullptr);
            config_ = std::move(other.config_);
            punctuationEnabled_ = other.punctuationEnabled_;
        }
        return *this;
    }

    ~OpenccJiebaHelper() noexcept { cleanup(); }

    // ---------------------------
    // Stateful conversion config
    // ---------------------------

    // Stores the exact config name for stateful convert() calls.
    //
    // Invalid names are intentionally preserved so the native C API can
    // surface its own "Invalid config: ..." behavior rather than silently
    // falling back to another config.
    void setConfig(const std::string_view config) {
        config_.assign(config.data(), config.size());
    }

    [[nodiscard]] const std::string &getConfig() const noexcept {
        return config_;
    }

    void setPunctuation(const bool enabled) noexcept {
        punctuationEnabled_ = enabled;
    }

    [[nodiscard]] bool punctuationEnabled() const noexcept {
        return punctuationEnabled_;
    }

    // ---------------------------
    // Conversion
    // ---------------------------

    // Converts using the stored config and punctuation flag.
    [[nodiscard]] std::string convert(const std::string_view input) const {
        if (input.empty()) return {};
        return convertByName(input, config_, punctuationEnabled_);
    }

    // Converts using an explicit config and punctuation setting without
    // changing the stored state.
    [[nodiscard]] std::string convert(
        const std::string_view input,
        const std::string_view config,
        const bool punctuation = false
    ) const {
        if (input.empty()) return {};
        return convertByName(input, config, punctuation);
    }

    // Checks whether input appears Simplified or Traditional Chinese.
    //
    // Returns:
    //   0 = mixed / undetermined
    //   1 = Traditional
    //   2 = Simplified
    //  -1 = native error
    [[nodiscard]] int zhoCheck(const std::string_view input) const {
        if (input.empty()) return 0;
        const std::string in(input);
        return opencc_jieba_zho_check(instance_, in.c_str());
    }

    // ---------------------------
    // Compatibility normalization
    // ---------------------------

    // Normalizes CJK Compatibility Ideographs.
    [[nodiscard]] std::string normalizeCompat(const std::string_view input) const {
        if (input.empty()) return {};
        return transformString(input, opencc_jieba_normalize_compat);
    }

    // Applies CJK compatibility normalization plus the curated extended
    // Unicode compatibility mappings.
    [[nodiscard]] std::string normalizeCompatExtended(const std::string_view input) const {
        if (input.empty()) return {};
        return transformString(input, opencc_jieba_normalize_compat_extended);
    }

    // Applies the built-in DeTofu display-compatibility fallback.
    [[nodiscard]] std::string detofu(
        const std::string_view input,
        const opencc_jieba_detofu_level_t level = OPENCC_JIEBA_DETOFU_EXT_B
    ) const {
        if (input.empty()) return {};

        const std::string in(input);
        char *output = opencc_jieba_detofu(instance_, in.c_str(), level);
        if (!output) return takeLastErrorText();

        std::string result(output);
        opencc_jieba_free_string(output);
        return result;
    }

    // ---------------------------
    // Jieba segmentation
    // ---------------------------

    [[nodiscard]] std::vector<std::string> cut(
        const std::string_view input,
        const bool hmm = true
    ) const {
        if (input.empty()) return {};

        const std::string in(input);
        char **result = opencc_jieba_cut(instance_, in.c_str(), hmm);
        return extractStringArray(result);
    }

    [[nodiscard]] std::vector<std::string> cutAll(
        const std::string_view input
    ) const {
        if (input.empty()) return {};

        const std::string in(input);
        char **result = opencc_jieba_cut_all(instance_, in.c_str());
        return extractStringArray(result);
    }

    [[nodiscard]] std::vector<std::string> cutForSearch(
        const std::string_view input,
        const bool hmm = true
    ) const {
        if (input.empty()) return {};

        const std::string in(input);
        char **result = opencc_jieba_cut_for_search(instance_, in.c_str(), hmm);
        return extractStringArray(result);
    }

    [[nodiscard]] std::string cutAndJoin(
        const std::string_view input,
        const bool hmm = true,
        const std::string_view delimiter = " "
    ) const {
        if (input.empty()) return {};

        const std::string in(input);
        const std::string delim(delimiter);

        char *output =
            opencc_jieba_cut_and_join(instance_, in.c_str(), hmm, delim.c_str());

        if (!output) return takeLastErrorText();

        std::string result(output);
        opencc_jieba_free_string(output);
        return result;
    }

    [[nodiscard]] std::vector<Tag> tag(
        const std::string_view input,
        const bool hmm = true
    ) const {
        if (input.empty()) return {};

        const std::string in(input);
        OpenccJiebaTag *array = opencc_jieba_tag(instance_, in.c_str(), hmm);
        if (!array) return {};

        try {
            std::vector<Tag> result;

            for (std::size_t i = 0;
                 array[i].word != nullptr || array[i].tag != nullptr;
                 ++i) {
                result.push_back({
                    array[i].word ? array[i].word : "",
                    array[i].tag ? array[i].tag : "",
                });
            }

            opencc_jieba_free_tag_array(array);
            return result;
        }
        catch (...) {
            opencc_jieba_free_tag_array(array);
            throw;
        }
    }

    // ---------------------------
    // Keyword extraction
    // ---------------------------

    [[nodiscard]] std::vector<std::string> extractKeywords(
        const std::string_view input,
        const std::size_t topK,
        const std::string_view method
    ) const {
        if (input.empty()) return {};

        const std::string in(input);
        const std::string methodOwned(method);

        char **result =
            opencc_jieba_keywords(instance_, in.c_str(), topK, methodOwned.c_str());

        return extractStringArray(result);
    }

    [[nodiscard]] std::vector<std::string> extractKeywords(
        const std::string_view input,
        const std::size_t topK,
        const std::string_view method,
        const std::string_view allowedPos
    ) const {
        if (input.empty()) return {};

        const std::string in(input);
        const std::string methodOwned(method);
        const std::string posOwned(allowedPos);

        char **result = opencc_jieba_keywords_pos(
            instance_,
            in.c_str(),
            topK,
            methodOwned.c_str(),
            posOwned.empty() ? nullptr : posOwned.c_str()
        );

        return extractStringArray(result);
    }

    [[nodiscard]] std::vector<std::string>
    extractKeywordsTextRank(const std::string_view input, const std::size_t topK) const {
        return extractKeywords(input, topK, "textrank");
    }

    [[nodiscard]] std::vector<std::string>
    extractKeywordsTfidf(const std::string_view input, const std::size_t topK) const {
        return extractKeywords(input, topK, "tfidf");
    }

    [[nodiscard]] std::vector<WeightedKeyword> extractKeywordsAndWeights(
        const std::string_view input,
        const std::size_t topK,
        const std::string_view method
    ) const {
        return extractKeywordsAndWeightsImpl(input, topK, method, {});
    }

    [[nodiscard]] std::vector<WeightedKeyword> extractKeywordsAndWeights(
        const std::string_view input,
        const std::size_t topK,
        const std::string_view method,
        const std::string_view allowedPos
    ) const {
        return extractKeywordsAndWeightsImpl(input, topK, method, allowedPos);
    }

    // ---------------------------
    // Native thread-local errors
    // ---------------------------

    // Returns and frees the calling thread's current native error string.
    //
    // Call on the same thread immediately after a failed C API call.
    [[nodiscard]] static std::string lastError() {
        char *err = opencc_jieba_last_error();
        if (!err) return {};

        std::string result(err);
        opencc_jieba_free_string(err);
        return result;
    }

    static void clearLastError() noexcept {
        opencc_jieba_clear_last_error();
    }

private:
    void *instance_ = nullptr;
    std::string config_ = "s2t";
    bool punctuationEnabled_ = false;

    using StringTransformFn = char *(*)(const void *, const char *);

    static void cleanupInstance(void *instance) noexcept {
        if (instance) {
            opencc_jieba_delete(instance);
        }
    }

    void cleanup() noexcept {
        cleanupInstance(instance_);
        instance_ = nullptr;
    }

    [[nodiscard]] static std::string takeLastErrorText() {
        return lastError();
    }

    [[nodiscard]] static void *requireInstance(
        void *instance,
        const char *fallbackMessage
    ) {
        if (instance) return instance;

        std::string error = lastError();
        if (error.empty() || error == "No error") {
            error = fallbackMessage;
        }

        throw std::runtime_error(error);
    }

    [[nodiscard]] static void *createDefault() {
        return requireInstance(
            opencc_jieba_new(),
            "Failed to initialize OpenCC-Jieba instance."
        );
    }

    [[nodiscard]] static std::vector<OpenccJiebaUserDictEntry>
    buildUserDictFfi(const std::vector<UserDictEntry> &entries) {
        std::vector<OpenccJiebaUserDictEntry> ffiEntries;
        ffiEntries.reserve(entries.size());

        for (const UserDictEntry &entry : entries) {
            ffiEntries.push_back({
                entry.word.c_str(),
                entry.freq,
                entry.tag.empty() ? nullptr : entry.tag.c_str(),
            });
        }

        return ffiEntries;
    }

    [[nodiscard]] static void *createWithUserDict(
        const std::vector<UserDictEntry> &entries
    ) {
        const auto ffiEntries = buildUserDictFfi(entries);

        return requireInstance(
            opencc_jieba_new_user_dict(
                ffiEntries.empty() ? nullptr : ffiEntries.data(),
                ffiEntries.size()
            ),
            "Failed to initialize OpenCC-Jieba instance with user dictionary."
        );
    }

    [[nodiscard]] static std::vector<std::vector<OpenccJiebaCustomPair>>
    buildCustomPairArrays(const std::vector<CustomDictSpec> &specs) {
        std::vector<std::vector<OpenccJiebaCustomPair>> ffiPairArrays;
        ffiPairArrays.reserve(specs.size());

        for (const CustomDictSpec &spec : specs) {
            std::vector<OpenccJiebaCustomPair> ffiPairs;
            ffiPairs.reserve(spec.pairs.size());

            for (const CustomPair &pair : spec.pairs) {
                ffiPairs.push_back({
                    pair.source.c_str(),
                    pair.target.c_str(),
                });
            }

            ffiPairArrays.push_back(std::move(ffiPairs));
        }

        return ffiPairArrays;
    }

    [[nodiscard]] static std::vector<OpenccJiebaCustomDictSpec>
    buildCustomSpecFfi(
        const std::vector<CustomDictSpec> &specs,
        const std::vector<std::vector<OpenccJiebaCustomPair>> &ffiPairArrays
    ) {
        std::vector<OpenccJiebaCustomDictSpec> ffiSpecs;
        ffiSpecs.reserve(specs.size());

        for (std::size_t i = 0; i < specs.size(); ++i) {
            const auto &ffiPairs = ffiPairArrays[i];

            ffiSpecs.push_back({
                specs[i].slot,
                specs[i].mode,
                ffiPairs.empty() ? nullptr : ffiPairs.data(),
                ffiPairs.size(),
            });
        }

        return ffiSpecs;
    }

    [[nodiscard]] static void *createWithCustomDicts(
        const std::vector<CustomDictSpec> &specs
    ) {
        const auto ffiPairArrays = buildCustomPairArrays(specs);
        const auto ffiSpecs = buildCustomSpecFfi(specs, ffiPairArrays);

        return requireInstance(
            opencc_jieba_new_custom(
                ffiSpecs.empty() ? nullptr : ffiSpecs.data(),
                ffiSpecs.size()
            ),
            "Failed to initialize OpenCC-Jieba instance with custom dictionaries."
        );
    }

    [[nodiscard]] static void *createWithUserDictAndCustom(
        const std::vector<UserDictEntry> &entries,
        const std::vector<CustomDictSpec> &specs
    ) {
        const auto ffiEntries = buildUserDictFfi(entries);
        const auto ffiPairArrays = buildCustomPairArrays(specs);
        const auto ffiSpecs = buildCustomSpecFfi(specs, ffiPairArrays);

        return requireInstance(
            opencc_jieba_new_user_dict_custom(
                ffiEntries.empty() ? nullptr : ffiEntries.data(),
                ffiEntries.size(),
                ffiSpecs.empty() ? nullptr : ffiSpecs.data(),
                ffiSpecs.size()
            ),
            "Failed to initialize OpenCC-Jieba instance with user and custom dictionaries."
        );
    }

    [[nodiscard]] std::string convertByName(
        const std::string_view input,
        const std::string_view config,
        const bool punctuation
    ) const {
        const std::string in(input);
        const std::string cfg(config);

        char *output =
            opencc_jieba_convert(instance_, in.c_str(), cfg.c_str(), punctuation);

        if (!output) return takeLastErrorText();

        std::string result(output);
        opencc_jieba_free_string(output);
        return result;
    }

    [[nodiscard]] std::string transformString(
        const std::string_view input,
        const StringTransformFn fn
    ) const {
        const std::string in(input);
        char *output = fn(instance_, in.c_str());

        if (!output) return takeLastErrorText();

        std::string result(output);
        opencc_jieba_free_string(output);
        return result;
    }

    [[nodiscard]] static std::vector<std::string>
    extractStringArray(char **array) {
        if (!array) return {};

        try {
            std::vector<std::string> result;

            for (std::size_t i = 0; array[i] != nullptr; ++i) {
                result.emplace_back(array[i]);
            }

            opencc_jieba_free_string_array(array);
            return result;
        }
        catch (...) {
            opencc_jieba_free_string_array(array);
            throw;
        }
    }

    [[nodiscard]] std::vector<WeightedKeyword> extractKeywordsAndWeightsImpl(
        const std::string_view input,
        const std::size_t topK,
        const std::string_view method,
        const std::string_view allowedPos
    ) const {
        if (input.empty()) return {};

        const std::string in(input);
        const std::string methodOwned(method);
        const std::string posOwned(allowedPos);

        std::size_t len = 0;
        char **keywords = nullptr;
        double *weights = nullptr;

        const int32_t code = posOwned.empty()
            ? opencc_jieba_keywords_and_weights(
                instance_,
                in.c_str(),
                topK,
                methodOwned.c_str(),
                &len,
                &keywords,
                &weights
            )
            : opencc_jieba_keywords_and_weights_pos(
                instance_,
                in.c_str(),
                topK,
                methodOwned.c_str(),
                posOwned.c_str(),
                &len,
                &keywords,
                &weights
            );

        if (code != 0) {
            std::string error = takeLastErrorText();
            if (error.empty() || error == "No error") {
                error = "Keyword extraction failed.";
            }
            throw std::runtime_error(error);
        }

        try {
            std::vector<WeightedKeyword> result;
            result.reserve(len);

            for (std::size_t i = 0; i < len; ++i) {
                result.push_back({
                    keywords[i] ? keywords[i] : "",
                    weights[i],
                });
            }

            opencc_jieba_free_keywords_and_weights(keywords, weights, len);
            return result;
        }
        catch (...) {
            opencc_jieba_free_keywords_and_weights(keywords, weights, len);
            throw;
        }
    }
};
