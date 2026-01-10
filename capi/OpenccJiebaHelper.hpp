#pragma once
#include <string>
#include <vector>
#include <stdexcept>
#include <algorithm>
#include <array>

#include "opencc_jieba_capi.h"

class OpenccJiebaHelper
{
public:
    // ---- Lifecycle ---------------------------------------------------------
    OpenccJiebaHelper()
        : instance_(opencc_jieba_new())
          , config_("s2t")
          , punctuation_(false)
    {
        if (!instance_)
        {
            throw std::runtime_error("Failed to initialize OpenCC-Jieba instance.");
        }
    }

    ~OpenccJiebaHelper()
    {
        if (instance_)
        {
            opencc_jieba_delete(instance_);
            instance_ = nullptr;
        }
    }

    OpenccJiebaHelper(const OpenccJiebaHelper&) = delete;
    OpenccJiebaHelper& operator=(const OpenccJiebaHelper&) = delete;

    OpenccJiebaHelper(OpenccJiebaHelper&& other) noexcept
        : instance_(other.instance_)
          , config_(std::move(other.config_))
          , punctuation_(other.punctuation_)
    {
        other.instance_ = nullptr;
    }

    OpenccJiebaHelper& operator=(OpenccJiebaHelper&& other) noexcept
    {
        if (this != &other)
        {
            // destroy ours
            if (instance_) opencc_jieba_delete(instance_);
            // steal
            instance_ = other.instance_;
            config_ = std::move(other.config_);
            punctuation_ = other.punctuation_;
            other.instance_ = nullptr;
        }
        return *this;
    }

    // ---- Configuration -----------------------------------------------------
    // Known-good OpenCC-style configs you want to allow here; tweak as needed.
    // Keep small, constexpr, and case-sensitive for zero heap overhead.
    static constexpr std::array<const char*, 16> kKnownConfigs{
        "s2t", "t2s", "s2tw", "tw2s", "s2twp", "tw2sp", "s2hk", "hk2s", "t2tw",
        "t2twp", "t2hk", "tw2t", "tw2tp", "hk2t", "t2jp", "jp2t"
    };

    static bool isValidConfig(const std::string& cfg)
    {
        return std::any_of(kKnownConfigs.begin(), kKnownConfigs.end(),
                           [&](const char* c) { return cfg == c; });
    }

    void setConfig(std::string cfg)
    {
        if (!isValidConfig(cfg))
        {
            throw std::invalid_argument("Invalid OpenCC config: " + cfg);
        }
        config_ = std::move(cfg);
    }

    [[nodiscard]] const std::string& getConfig() const noexcept { return config_; }

    void setPunctuation(const bool enabled) noexcept { punctuation_ = enabled; }

    [[nodiscard]] bool punctuationEnabled() const noexcept { return punctuation_; }

    // ---- Conversion --------------------------------------------------------
    // Uses stored config_ + punctuation_
    [[nodiscard]] std::string convert(const std::string& input) const
    {
        if (input.empty()) return {};

        char* output = opencc_jieba_convert(instance_, input.c_str(), config_.c_str(), punctuation_);
        if (!output) return {};

        std::string result(output);
        opencc_jieba_free_string(output);
        return result;
    }

    // Ad-hoc override (does not modify stored settings)
    [[nodiscard]] std::string convert(const std::string& input,
                                      const std::string& cfgOverride,
                                      const bool punctOverride) const
    {
        if (input.empty()) return {};

        const std::string& cfg = isValidConfig(cfgOverride) ? cfgOverride : config_;
        char* output = opencc_jieba_convert(instance_, input.c_str(), cfg.c_str(), punctOverride);
        if (!output) return {};

        std::string result(output);
        opencc_jieba_free_string(output);
        return result;
    }

    // ---- Utilities ---------------------------------------------------------
    [[nodiscard]] int zhoCheck(const std::string& input) const
    {
        return input.empty() ? 0 : opencc_jieba_zho_check(instance_, input.c_str());
    }

    [[nodiscard]] std::vector<std::string> cut(const std::string& input, const bool hmm = true) const
    {
        char** result = opencc_jieba_cut(instance_, input.c_str(), hmm);
        return extractStringArray(result);
    }

    [[nodiscard]] std::vector<std::string> extractKeywords(const std::string& input,
                                                           const int topK,
                                                           const std::string& method) const
    {
        char** result = opencc_jieba_keywords(instance_, input.c_str(), topK, method.c_str());
        return extractStringArray(result);
    }

    [[nodiscard]] std::string cutAndJoin(const std::string& input,
                                         const bool hmm = true,
                                         const std::string& delimiter = " ") const
    {
        if (input.empty()) return {};
        char* output = opencc_jieba_cut_and_join(instance_, input.c_str(), hmm, delimiter.c_str());
        if (!output) return {};

        std::string result(output);
        opencc_jieba_free_string(output);
        return result;
    }

    [[nodiscard]] std::vector<std::string> extractKeywordsTextRank(const std::string& input, const int topK) const
    {
        return extractKeywords(input, topK, "textrank");
    }

    [[nodiscard]] std::vector<std::string> extractKeywordsTfidf(const std::string& input, const int topK) const
    {
        return extractKeywords(input, topK, "tfidf");
    }

    [[nodiscard]] std::pair<std::vector<std::string>, std::vector<double>>
    extractKeywordsAndWeights(const std::string& input, const int topK, const std::string& method) const
    {
        size_t len = 0;
        char** keywords = nullptr;
        double* weights = nullptr;

        const int code = opencc_jieba_keywords_and_weights(
            instance_, input.c_str(), topK, method.c_str(), &len, &keywords, &weights
        );

        if (code != 0)
        {
            throw std::runtime_error("Keyword extraction failed.");
        }

        std::vector<std::string> keywordList;
        std::vector<double> weightList;
        keywordList.reserve(len);
        weightList.reserve(len);

        for (size_t i = 0; i < len; ++i)
        {
            keywordList.emplace_back(keywords[i]);
            weightList.emplace_back(weights[i]);
        }

        opencc_jieba_free_keywords_and_weights(keywords, weights, len);
        return {std::move(keywordList), std::move(weightList)};
    }

private:
    void* instance_{nullptr};
    std::string config_; // persistent OpenCC config for convert()
    bool punctuation_; // persistent punctuation flag for convert()

    static std::vector<std::string> extractStringArray(char** array)
    {
        std::vector<std::string> result;
        if (!array) return result;

        for (size_t i = 0; array[i] != nullptr; ++i)
        {
            result.emplace_back(array[i]);
        }
        opencc_jieba_free_string_array(array);
        return result;
    }
};
