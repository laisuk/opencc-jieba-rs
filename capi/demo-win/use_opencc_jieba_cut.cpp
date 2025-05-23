#include <iostream>
#include <windows.h>
#include "opencc_jieba_capi.h"

int main() {
    SetConsoleOutputCP(65001);
    // Create OpenCC instance
    void *instance = opencc_jieba_new();
    // Input string
    const char *input = "你好，美丽的世界！";
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
    opencc_jieba_delete(instance);

    return 0;
}
