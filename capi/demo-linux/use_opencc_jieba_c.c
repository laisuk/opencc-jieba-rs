#include <stdio.h>
#include "opencc_jieba_capi.h"

int main(int argc, char **argv) {
    void *opencc = opencc_new();
    const char *config = u8"s2twp";
    const char *text = u8"意大利罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";
    printf("Text: %s\n", text);
    int code = opencc_zho_check(opencc, text);
    printf("Text Code: %d\n", code);
    char *result = opencc_convert(opencc, text, config, true);
    code = opencc_zho_check(opencc, result);
    printf("Converted: %s\n", result);
    printf("Text Code: %d\n", code);
    if (result != NULL) {
        opencc_string_free(result);
    }
    if (opencc != NULL) {
        opencc_free(opencc);
    }

    return 0;
}
