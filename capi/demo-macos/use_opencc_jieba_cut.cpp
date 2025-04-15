#include <iostream>
#include "opencc_jieba_capi.h"

int main() {
    // Create OpenCC instance
    void *instance = opencc_jieba_new();
    // Input string
    const char *input = "你好，世界！";
    // Delimiter
    const char *delimiter = "/ ";
    // Perform segmentation and join with delimiter
    char *result = opencc_jieba_cut_and_join(instance, input, false, delimiter);
    if (result != NULL) {
        // Print the result
        std::cout << "Result: " << result << std::endl;
        // Free memory
        opencc_jieba_free_string(result);
    }
    // Free OpenCC instance
    opencc_jieba_free(instance);

    return 0;
}
