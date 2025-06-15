#pragma once
#include <string>
#include <vector>
#include <stdexcept>
#include "opencc_jieba_capi.h"

class OpenccJiebaHelper
{
public:
    OpenccJiebaHelper()
    {
        instance_ = opencc_jieba_new();
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
        }
    }

    [[nodiscard]] std::string convert(const std::string& input, const std::string& config = "s2t",
                                      const bool punctuation = false) const
    {
        if (input.empty()) return "";

        char* output = opencc_jieba_convert(instance_, input.c_str(), config.c_str(), punctuation);
        if (!output) return "";

        std::string result(output);
        opencc_jieba_free_string(output);
        return result;
    }

    [[nodiscard]] int zhoCheck(const std::string& input) const
    {
        return input.empty() ? 0 : opencc_jieba_zho_check(instance_, input.c_str());
    }

    [[nodiscard]] std::vector<std::string> cut(const std::string& input, const bool hmm = true) const
    {
        char** result = opencc_jieba_cut(instance_, input.c_str(), hmm);
        return extractStringArray(result);
    }

    [[nodiscard]] std::vector<std::string> extractKeywords(const std::string& input, const int topK,
                                                           const std::string& method) const
    {
        char** result = opencc_jieba_keywords(instance_, input.c_str(), topK, method.c_str());
        return extractStringArray(result);
    }

    [[nodiscard]] std::string cutAndJoin(const std::string& input, const bool hmm = true,
                                         const std::string& delimiter = " ") const
    {
        if (input.empty()) return "";

        char* output = opencc_jieba_cut_and_join(instance_, input.c_str(), hmm, delimiter.c_str());
        if (!output) return "";

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

    [[nodiscard]] std::pair<std::vector<std::string>, std::vector<double>> extractKeywordsAndWeights(
        const std::string& input, const int topK, const std::string& method
    ) const
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

        for (size_t i = 0; i < len; ++i)
        {
            keywordList.emplace_back(keywords[i]);
            weightList.push_back(weights[i]);
        }

        opencc_jieba_free_keywords_and_weights(keywords, weights, len);
        return {keywordList, weightList};
    }

private:
    void* instance_;

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
