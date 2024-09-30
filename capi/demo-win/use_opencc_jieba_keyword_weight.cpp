#include <iostream>
#include <windows.h>
#include "opencc_jieba_capi.h"

int main() {
    SetConsoleOutputCP(65001);
    // Create OpenCC instance
    void *instance = opencc_new();
    // Input string
    const char *input = "该剧讲述三位男女在平安夜这一天各自的故事。平安夜的0点，横滨山下码头发生枪杀事件。胜吕寺诚司（二宫和也）在码头的一个角落醒来，眼前躺着一具头部被击中的尸体，失去记忆的他成为了被警察追赶的逃犯。";
    // Perform keyword extraction using TextRank
    size_t top_k = 10;  // Specify the number of top keywords to extract
    size_t keyword_count = 0; // Variable to store the count of keywords
    char **keywords = nullptr; // Pointer for keyword strings
    double *weights = nullptr; // Pointer for weights

    int32_t result = opencc_jieba_keyword_weight_textrank(
        (const struct OpenCC*)instance,
        input,
        top_k,
        &keyword_count,
        &keywords,
        &weights
    );
    // Check if the keyword extraction was successful
    if (result == 0) {
        std::cout << "Keyword Extraction Successful! Number of Keywords: " << keyword_count << "\n";
        // Print keywords and their weights
        for (size_t i = 0; i < keyword_count; ++i) {
            std::cout << "Keyword: " << keywords[i] << ", Weight: " << weights[i] << "\n";
        }
        // Free memory allocated for keywords and weights
        opencc_jieba_free_keywords_and_weights(keywords, weights, keyword_count);
    } else {
        std::cerr << "Keyword extraction failed with error code: " << result << "\n";
    }
    // Perform segmentation and join with delimiter
    char **result_segments = opencc_jieba_keyword_extract_textrank(instance, input, 10);
    char *join_result = opencc_join_str(result_segments, "/");
    std::cout << "Joined output: " << join_result << "\n";
    opencc_string_free(join_result);

    if (result_segments != NULL) {
        // Print the segmentation result
        std::cout << "Segmentation Result: ";
        for (int i = 0; result_segments[i] != NULL; ++i) {
            std::cout << result_segments[i];
            if (result_segments[i + 1] != NULL) {
                std::cout << "/ ";
            }
        }
        std::cout << std::endl;
        // Free memory
        opencc_free_string_array(result_segments);
    }
    // Free OpenCC instance
    opencc_free(instance);

    return 0;
}
