#include <iostream>
#include "opencc_jieba_capi.h"

int main() {
    // Create OpenCC instance
    void *instance = opencc_new();
    // Input string
    const char *input = "你好，世界！";
    // Delimiter
    const char *delimiter = "/ ";
    // Perform segmentation and join with delimiter
    char *result = opencc_jieba_cut_and_join(instance, input, true, delimiter);
    if (result != NULL) {
        // Print the result
        std::cout << "Result: " << result << std::endl;
        // Free memory
        opencc_string_free(result);
    }
    // Free OpenCC instance
    opencc_free(instance);

    return 0;
}
