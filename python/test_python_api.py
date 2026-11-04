"""Regression checks for the Python C ABI facade and CLI.

Run: python -m unittest discover -s python -p test_python_api.py -v
"""
import contextlib
import io
import tempfile
import unittest
import zipfile
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from opencc_jieba_rs import OpenCC
from opencc_jieba_rs.__main__ import main, parse_custom_dict_spec


class PythonApiTests(unittest.TestCase):
    def test_config_and_normalization(self):
        cc = OpenCC('T2S')
        self.assertEqual(cc.convert('漢語'), '汉语')
        with self.assertRaises(ValueError):
            cc.config = 'invalid'
        cc.set_config('invalid')
        self.assertEqual(cc.config, 's2t')
        self.assertEqual(cc.normalize_compat('神'), '神')
        self.assertEqual(cc.normalize_unicode_compat('神聼'), '神聽')
        with self.assertRaises(ValueError):
            cc.convert('a\0b')

    def test_customization_replay_and_atomic_failure(self):
        cc = OpenCC.from_dicts(specs=[{'slot': ' stcharacters.TXT ', 'mode': ' APPEND ', 'pairs': [('猫', '狗')]}])
        cc.add_word('量子测试词', 100000, 'nz')
        cc.load_custom_dicts([{'slot': 'STCharacters', 'pairs': [('狗', '牛')]}])
        self.assertEqual(cc.convert('猫狗'), '狗牛')
        self.assertEqual(cc.jieba_tag('量子测试词'), [('量子测试词', 'nz')])
        with self.assertRaises(ValueError):
            cc.load_user_dict_entries([{'word': 'bad word', 'freq': 10}])
        self.assertEqual(cc.convert('猫狗'), '狗牛')
        cc.load_custom_dicts([{'slot': 'STCharacters', 'mode': 'override', 'pairs': [('猫', '羊')]}])
        self.assertEqual(cc.convert('猫狗'), '羊狗')

    def test_dictionary_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp)
            mappings = path / 'mapping.txt'
            mappings.write_text('\ufeff# mappings\n猫\t狗 犬\n', encoding='utf-8')
            words = path / 'words.txt'
            words.write_text('量子测试词 100000 nz\n', encoding='utf-8')
            cc = OpenCC.from_user_dict_files(files=[str(words)])
            cc.load_custom_dict_files([{'slot': 'STCharacters', 'files': [str(mappings)]}])
            self.assertEqual(cc.convert('猫'), '狗')
            self.assertEqual(cc.jieba_cut('量子测试词'), ['量子测试词'])
            words.write_text('broken\n', encoding='utf-8')
            with self.assertRaises(ValueError):
                cc.load_user_dict(str(words))
            self.assertEqual(cc.convert('猫'), '狗')

    def test_detofu_custom_precedence_and_threshold(self):
        cc = OpenCC()
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'fallback.txt'
            path.write_text('𣭲\tＡ\tExtB\n猫\t犬\tExtI\n', encoding='utf-8')
            self.assertEqual(cc.detofu_with_custom_file('𣭲猫', 'all', str(path)), 'Ａ犬')
            self.assertEqual(cc.detofu_with_custom_file('𣭲猫', 'ExtI', str(path)), '𣭲犬')
        self.assertEqual(cc.detofu('𣭲', 'all'), '氄')
        with self.assertRaises(ValueError):
            cc.detofu('text', 'ExtJ')

    def test_segmentation_and_keywords(self):
        cc = OpenCC()
        text = '中国科学院的科学家正在研究人工智能。'
        for mode in ('cut', 'search', 'full', 'tag'):
            self.assertTrue(cc.jieba_segment_join(text, mode=mode))
        self.assertEqual(cc.jieba_segment_join(text, delim=''), text)
        self.assertEqual(cc.jieba_cut_and_join(text, True, '/'), cc.jieba_cut_and_join(text, '/'))
        for algorithm in ('textrank', 'tfidf'):
            extract = getattr(cc, 'jieba_keyword_extract_' + algorithm)
            weighted = getattr(cc, 'jieba_keyword_weight_' + algorithm)
            words = extract(text, 5, ['n', 'nz'])
            self.assertEqual(words, [word for word, _ in weighted(text, 5, ['n', 'nz'])])
            self.assertEqual(extract('', 0), [])
        self.assertEqual(cc.jieba_join_str(['', '中国'], '/'), '/中国')


class CliTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def run_cli(self, args):
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()) as err:
            status = main(args)
        return status, err.getvalue()

    def test_convert_segment_and_errors(self):
        source, target = self.root / 'input.txt', self.root / 'output.txt'
        source.write_text('汉语神', encoding='utf-8')
        self.assertEqual(self.run_cli(['convert', '-i', str(source), '-o', str(target), '-E', '--detofu'])[0], 0)
        self.assertEqual(target.read_text(encoding='utf-8'), '漢語神')
        self.assertEqual(self.run_cli(['segment', '-i', str(source), '-o', str(target), '-d', ''])[0], 0)
        self.assertEqual(target.read_text(encoding='utf-8'), '汉语神')
        self.assertEqual(self.run_cli(['convert', '-i', str(source), '-o', str(source)])[0], 1)
        self.assertEqual(source.read_text(encoding='utf-8'), '汉语神')
        self.assertEqual(self.run_cli(['convert', '-c', 'invalid'])[0], 1)
        self.assertEqual(self.run_cli(['convert', '--detofu-file', 'missing'])[0], 1)
        self.assertEqual(self.run_cli(['convert', '-i', str(source), '--detofu', '--detofu-file', 'missing'])[0], 1)

    def test_office_pipeline_and_filename(self):
        source = self.root / '汉语.docx'
        with zipfile.ZipFile(source, 'w') as package:
            package.writestr('word/document.xml', '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:r><w:t>汉语神</w:t></w:r></w:p></w:document>')
        status, message = self.run_cli(['office', '-i', str(source), '-c', 's2t', '-F', '-E', '-k'])
        self.assertEqual(status, 0, message)
        output = self.root / '漢語_converted.docx'
        with zipfile.ZipFile(output) as package:
            self.assertIn('漢語神', package.read('word/document.xml').decode('utf-8'))
        self.assertEqual(self.run_cli(['office', '-i', str(source), '-o', str(source)])[0], 1)

    def test_windows_dictionary_path_parser(self):
        path = self.root / 'dict.txt'
        path.write_text('猫\t狗\n', encoding='utf-8')
        spec = parse_custom_dict_spec(f'STCharacters:append:{path}')
        self.assertEqual(spec['files'], [str(path)])


if __name__ == '__main__':
    unittest.main()
