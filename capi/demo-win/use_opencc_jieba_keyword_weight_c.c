#include <stdio.h>
#include <stdlib.h>
#include <windows.h>
#include "opencc_jieba_capi.h"

int main() {
    SetConsoleOutputCP(65001);
    // Create OpenCC instance
    void *instance = opencc_jieba_new();
    if (instance == NULL) {
        fprintf(stderr, "Failed to create OpenCC instance.\n");
        return -1;
    }

    // Input string
    const char *input = "该剧讲述三位男女在平安夜这一天各自的故事。平安夜的0点，横滨山下码头发生枪杀事件。胜吕寺诚司（二宫和也）在码头的一个角落醒来，眼前躺着一具头部被击中的尸体，失去记忆的他成为了被警察追赶的逃犯。";
    size_t top_k = 10;  // Specify the number of top keywords to extract
    const char *method = "textrank";
    size_t keyword_count = 0;
    char **keywords = NULL;
    double *weights = NULL;

    int32_t result = opencc_jieba_keywords_and_weights(
        instance,
        input,
        top_k,
        method,
        &keyword_count,
        &keywords,
        &weights
    );

    if (result == 0) {
        printf("Keyword Extraction Successful! Number of Keywords: %zu\n", keyword_count);
        for (size_t i = 0; i < keyword_count; ++i) {
            printf("Keyword: %s, Weight: %f\n", keywords[i], weights[i]);
        }
        opencc_jieba_free_keywords_and_weights(keywords, weights, keyword_count);
    } else {
        fprintf(stderr, "Keyword extraction failed with error code: %d\n", result);
    }

    // Perform segmentation and join with delimiter
    char **result_segments = opencc_jieba_keywords(instance, input, 10, "textrank");
    char *join_result = opencc_jieba_join_str(result_segments, "/");
    printf("Joined output: %s\n", join_result);
    opencc_jieba_free_string(join_result);

    if (result_segments != NULL) {
        printf("Segmentation Result: ");
        for (int i = 0; result_segments[i] != NULL; ++i) {
            printf("%s", result_segments[i]);
            if (result_segments[i + 1] != NULL) {
                printf("/ ");
            }
        }
        printf("\n");
        opencc_jieba_free_string_array(result_segments);
    }

    opencc_jieba_free(instance);

    return 0;
}
