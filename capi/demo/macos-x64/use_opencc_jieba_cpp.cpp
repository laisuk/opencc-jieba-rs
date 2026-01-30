#include <iostream>
#include "opencc_jieba_capi.h"

int main(int argc, char **argv) {
    auto opencc = opencc_jieba_new();
    const char *config = u8"s2twp";
    const char *text = u8"意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
    std::cout << "Text: " << text << "\n";
    auto code = opencc_jieba_zho_check(opencc, text);
    std::cout << "Text Code: " << code << "\n";
    char *result = opencc_jieba_convert(opencc, text, config, true);
    code = opencc_jieba_zho_check(opencc, result);
    std::cout << "Converted: " << result << "\n";
    std::cout << "Text Code: " << code << "\n";
    if (result != NULL) {
        opencc_jieba_free_string(result);
    }
    if (opencc != NULL) {
        opencc_jieba_delete(opencc);
    }

    return 0;
}