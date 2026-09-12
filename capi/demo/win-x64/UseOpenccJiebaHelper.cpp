// UseOpenccJiebaHelper.cpp

#include <iostream>
#include <string>
#include <vector>

#ifdef _WIN32
#include <windows.h>
#endif

#include "OpenccJiebaHelper.hpp"

int main(int argc, char** argv)
{
    (void)argc;
    (void)argv;

#ifdef _WIN32
    // Enable UTF-8 output on Windows console.
    SetConsoleOutputCP(65001);
#endif

    try
    {
        OpenccJiebaHelper helper;

        const std::string text =
            u8"意大利邻国法兰西罗浮宫里收藏的“蒙娜丽莎的微笑”画像是旷世之作。";

        std::cout << "Text: " << text << "\n";
        std::cout << "Text Code: " << helper.zhoCheck(text) << "\n";

        // -------------------------------------------------------------
        // Test 1: Stateful string config conversion
        // -------------------------------------------------------------
        std::cout << "\n== Test 1: stateful conversion ==\n";

        helper.setConfig("s2twp");
        helper.setPunctuation(true);

        const std::string traditional = helper.convert(text);

        std::cout << "Converted: " << traditional << "\n";
        std::cout << "Converted Code: " << helper.zhoCheck(traditional) << "\n";

        // -------------------------------------------------------------
        // Test 2: Stateless string-config compatibility API
        // -------------------------------------------------------------
        std::cout << "\n== Test 2: explicit string config ==\n";

        const std::string byName =
            helper.convert(text, "s2twp", true);

        std::cout << "Converted: " << byName << "\n";
        std::cout << "Same result: "
                  << (byName == traditional ? "PASS" : "FAIL")
                  << "\n";

        const std::string invalid =
            helper.convert(text, "what_is_this", false);

        std::cout << "Invalid config returned: " << invalid << "\n";

        // -------------------------------------------------------------
        // Test 3: Jieba user dictionary + custom conversion dictionary
        // -------------------------------------------------------------
        std::cout
            << "\n== Test 3: Jieba user dictionary + custom dictionary roundtrip ==\n";

        const std::vector<OpenccJiebaHelper::UserDictEntry> userEntries = {
            {u8"帕兰蒂尔", 100000, ""},
            {u8"柏蘭蒂爾", 100000, ""},
            {u8"软件", 100000, ""},
            {u8"軟體", 100000, ""},
        };

        const std::vector<OpenccJiebaHelper::CustomDictSpec> customDicts = {
            {
                OPENCC_JIEBA_DICT_SLOT_ST_PHRASES,
                OPENCC_JIEBA_CUSTOM_DICT_APPEND,
                {
                    {u8"帕兰蒂尔", u8"柏蘭蒂爾"},
                    {u8"软件", u8"軟體"},
                }
            },
            {
                OPENCC_JIEBA_DICT_SLOT_TS_PHRASES,
                OPENCC_JIEBA_CUSTOM_DICT_APPEND,
                {
                    {u8"柏蘭蒂爾", u8"帕兰蒂尔"},
                    {u8"軟體", u8"软件"},
                }
            }
        };

        const OpenccJiebaHelper customHelper(userEntries, customDicts);

        const std::string roundtripSource =
            u8"帕兰蒂尔是一家软件公司。";

        const std::string roundtripTraditional =
            customHelper.convert(roundtripSource, "s2t", false);

        const std::string roundtripSimplified =
            customHelper.convert(roundtripTraditional, "t2s", false);

        std::cout << "Source:      " << roundtripSource << "\n";
        std::cout << "S2T custom:  " << roundtripTraditional << "\n";
        std::cout << "T2S custom:  " << roundtripSimplified << "\n";
        std::cout << "Roundtrip:   "
                  << (roundtripSimplified == roundtripSource ? "PASS" : "FAIL")
                  << "\n";

        // -------------------------------------------------------------
        // Test 4: Compatibility normalization
        // -------------------------------------------------------------
        std::cout << "\n== Test 4: compatibility normalization ==\n";

        const std::string compatSource =
            u8"天龍八部書";

        const std::string compatNormalized =
            helper.normalizeCompat(compatSource);

        const bool compatPass =
            compatNormalized == u8"天龍八部書";

        std::cout << "Source:       " << compatSource << "\n";
        std::cout << "Norm compat:  " << compatNormalized << "\n";
        std::cout << "Result:       "
                  << (compatPass ? "PASS" : "FAIL")
                  << "\n";

        // -------------------------------------------------------------
        // Test 5: Extended normalization -> conversion
        // -------------------------------------------------------------
        std::cout << "\n== Test 5: extended normalization -> T2S ==\n";

        const std::string extendedSource =
            u8"天龍八部書裡的聼眾‧聼聼竒羙⽟䂖甁噐⾳";

        const std::string normalized =
            helper.normalizeCompatExtended(extendedSource);

        const std::string simplified =
            helper.convert(normalized, "t2s", false);

        const bool normPass =
            normalized == u8"天龍八部書裡的聽眾·聽聽奇美玉石瓶器音";

        const bool t2sPass =
            simplified == u8"天龙八部书里的听众·听听奇美玉石瓶器音";

        std::cout << "Source:         " << extendedSource << "\n";
        std::cout << "Norm extended:  " << normalized << "\n";
        std::cout << "T2S:            " << simplified << "\n";
        std::cout << "Pipeline:       "
                  << (normPass && t2sPass ? "PASS" : "FAIL")
                  << "\n";

        // -------------------------------------------------------------
        // Test 6: DeTofu post-processing
        // -------------------------------------------------------------
        std::cout << "\n== Test 6: DeTofu ExtB ==\n";

        const std::string detofuSource =
            u8"骖𬴂";

        const std::string detofued =
            helper.detofu(detofuSource, OPENCC_JIEBA_DETOFU_EXT_B);

        const bool detofuPass =
            detofued == u8"骖騑";

        std::cout << "Source:      " << detofuSource << "\n";
        std::cout << "DeTofu:      " << detofued << "\n";
        std::cout << "Result:      "
                  << (detofuPass ? "PASS" : "FAIL")
                  << "\n";

        // -------------------------------------------------------------
        // Test 7: Recommended full compatibility pipeline
        // -------------------------------------------------------------
        std::cout << "\n== Test 7: normalize -> convert -> DeTofu ==\n";

        const std::string pipelineSource =
            u8"天龍八部書裡的聼眾，儼驂騑於上路。";

        const std::string pipelineNormalized =
            helper.normalizeCompatExtended(pipelineSource);

        const std::string pipelineConverted =
            helper.convert(pipelineNormalized, "t2s", false);

        const std::string pipelineDisplay =
            helper.detofu(
                pipelineConverted,
                OPENCC_JIEBA_DETOFU_EXT_B
            );

        std::cout << "Source:      " << pipelineSource << "\n";
        std::cout << "Normalized:  " << pipelineNormalized << "\n";
        std::cout << "Converted:   " << pipelineConverted << "\n";
        std::cout << "Display:     " << pipelineDisplay << "\n";

        std::cout << "\nAll tests completed.\n";
    }
    catch (const std::exception& ex)
    {
        std::cerr << "Exception: " << ex.what() << "\n";
        return 1;
    }

    return 0;
}
