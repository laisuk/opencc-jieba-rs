#include <iostream>
#include <windows.h>
#include "opencc_jieba_capi.h"

int main() {
    SetConsoleOutputCP(65001);
    // Create OpenCC instance
    void *instance = opencc_jieba_new();
    // Input string
    const char *input = "该剧讲述三位男女在平安夜这一天各自的故事。平安夜的0点，横滨山下码头发生枪杀事件。胜吕寺诚司（二宫和也）在码头的一个角落醒来，眼前躺着一具头部被击中的尸体，失去记忆的他成为了被警察追赶的逃犯。";
    // Perform segmentation and join with delimiter
    char **result = opencc_jieba_keywords(instance, input, 10, "textrank");

    char *join_result = opencc_jieba_join_str(result, "/");
        std::cout << "Joined output: " << join_result << "\n";
        opencc_jieba_free_string(join_result);

    if (result != NULL) {
            // Print the result
            std::cout << "Result: ";
            for (int i = 0; result[i] != NULL; ++i) {  // Iterate until we find a null pointer
                std::cout << result[i];  // Print each C string
                if (result[i + 1] != NULL) {  // Check if next string is not null
                    std::cout << "/ ";  // Print delimiter if not the last element
                }
            }
            std::cout << std::endl;  // New line after printing all results
            // Free memory
            opencc_jieba_free_string_array(result);  // Assuming you have a function to free the array
    }
    // Free OpenCC instance
    opencc_jieba_free(instance);

    return 0;
}
