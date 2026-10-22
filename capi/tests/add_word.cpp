#include "OpenccJiebaHelper.hpp"

#include <cassert>
#include <iostream>
#include <type_traits>

static_assert(std::is_same_v<decltype(&opencc_jieba_add_word),
                           bool (*)(void *, const char *, bool, size_t, const char *)>);

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
    std::cout << "C++ addWord regression passed\n";
}
