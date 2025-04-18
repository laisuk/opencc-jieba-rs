#include <stdio.h>
#include "opencc_jieba_capi.h"

int main(int argc, char **argv) {
    void *opencc = opencc_jieba_new();
    const char *config = u8"s2twp";
    const char *text = u8"意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
    printf("Text: %s\n", text);
    int code = opencc_jieba_zho_check(opencc, text);
    printf("Text Code: %d\n", code);
    char *result = opencc_jieba_convert(opencc, text, config, true);
    code = opencc_jieba_zho_check(opencc, result);
    printf("Converted: %s\n", result);
    printf("Converted Code: %d\n", code);
    if (result != NULL) {
        opencc_jieba_free_string(result);
    }
    if (opencc != NULL) {
        opencc_jieba_free(opencc);
    }

    return 0;
}
