#include "OpenccJiebaHelper.hpp"

#include <cassert>
#include <iostream>
#include <type_traits>

static_assert(std::is_same_v<decltype(&opencc_jieba_add_word),
                           bool (*)(void *, const char *, bool, size_t, const char *)>);

static_assert(std::is_same_v<decltype(&opencc_jieba_add_word_extended),
                           decltype(&opencc_jieba_add_word)>);

int main() {
    OpenccJiebaHelper helper;
    const std::string word = u8"星际量子咖啡馆";
    assert(helper.addWord(word, 100000, "nz"));
    assert(helper.cut(word, false) == std::vector<std::string>{word});
    const auto tags = helper.tag(word, false);
    assert(tags.size() == 1 && tags[0].tag == "nz");
    assert(helper.addWord(word));
    assert(helper.cut(word, false) == std::vector<std::string>{word});
    assert(helper.addWord(word, 0));
    assert(helper.cut(word, false) != std::vector<std::string>{word});
    assert(!helper.addWord(""));
    assert(OpenccJiebaHelper::lastError().find("word must not") != std::string::npos);
    assert(helper.addWord(word, std::nullopt, ""));
    assert(OpenccJiebaHelper::lastError() == "No error");
    assert(helper.cut(word, false) == std::vector<std::string>{word});
    const std::string traditional = u8"銀河草莓引擎";
    const std::string simplified = u8"银河草莓引擎";
    assert(!helper.addWordExtended(""));
    assert(OpenccJiebaHelper::lastError().find("word must not") != std::string::npos);
    assert(helper.addWordExtended(traditional, 100000, ""));
    assert(OpenccJiebaHelper::lastError() == "No error");
    assert(helper.cut(traditional, false) == std::vector<std::string>{traditional});
    assert(helper.cut(simplified, false) == std::vector<std::string>{simplified});
    std::cout << "C++ word registration regression passed\n";
}
