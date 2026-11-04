"""OpenCC + Jieba Python API backed by the bundled C ABI library.

Dictionary mutations and other calls on one instance require external synchronization.
"""
import ctypes as C
from pathlib import Path
from .opencc_jieba_rs import OpenCC as _LegacyOpenCC

from typing import List, Tuple, Optional, TypedDict

_CustomDictPair = Tuple[str, str]


class _CustomDictSpecRequired(TypedDict):
    slot: str
    pairs: List[_CustomDictPair]


class CustomDictSpec(_CustomDictSpecRequired, total=False):
    """In-memory OpenCC replacements for one dictionary slot.

    Required keys are ``slot`` (see ``OpenCC.available_slots()``) and ``pairs``
    (a list of source/target string tuples). Optional ``mode`` is ``"append"``
    by default, or ``"override"`` to clear that slot before inserting pairs.
    This defines conversion mappings, not Jieba word frequencies or POS tags.
    """
    mode: str


class _CustomDictFileSpecRequired(TypedDict):
    slot: str
    files: List[str]


class CustomDictFileSpec(_CustomDictFileSpecRequired, total=False):
    """UTF-8 OpenCC mapping files to load into one dictionary slot.

    Required keys are ``slot`` and ``files`` (a list of paths). Each file uses
    ``source<TAB>target`` lines, with an actual tab between the fields.
    Optional ``mode`` is ``"append"`` (default) or ``"override"``; override
    replaces the selected slot with the combined mappings from the files.
    These files have a different format from Jieba user dictionaries.
    """
    mode: str


class _UserDictEntryRequired(TypedDict):
    word: str
    freq: int


class UserDictEntry(_UserDictEntryRequired, total=False):
    """A Jieba word entry for segmentation and part-of-speech tagging.

    Required keys are ``word`` (a nonempty word without whitespace) and ``freq``
    (a non-negative integer that fits the platform's Rust ``usize``). Optional
    ``tag`` supplies a POS tag such as ``"nz"``. This registers a word without
    defining a Simplified/Traditional replacement; use ``CustomDictSpec`` for
    conversion mappings. Use ``add_word()`` to choose frequency automatically.
    """
    tag: str



class _Pair(C.Structure):
    _fields_ = [("source", C.c_char_p), ("target", C.c_char_p)]


class _Spec(C.Structure):
    _fields_ = [("slot", C.c_uint32), ("mode", C.c_uint32),
                ("pairs", C.POINTER(_Pair)), ("pair_count", C.c_size_t)]


class _Entry(C.Structure):
    _fields_ = [("word", C.c_char_p), ("freq", C.c_size_t), ("tag", C.c_char_p)]


def _utf8(value):
    if not isinstance(value, str):
        raise TypeError("Expected a string")
    if "\0" in value:
        raise ValueError("Embedded NUL characters are not supported by the C API")
    return value.encode("utf-8")


def _frequency(value):
    if not isinstance(value, int):
        raise TypeError("Frequency must be an integer")
    if not 0 <= value <= C.c_size_t(-1).value:
        raise ValueError("Frequency must fit a non-negative size_t")
    return value


_SLOTS = (
    "STCharacters STPhrases TSCharacters TSPhrases TWPhrases TWPhrasesRev "
    "HKPhrases HKPhrasesRev TWVariants TWVariantsPhrases TWVariantsRev "
    "TWVariantsRevPhrases HKVariants HKVariantsPhrases HKVariantsRev "
    "HKVariantsRevPhrases JPSCharacters JPSCharactersRev JPSPhrases"
).split()


class OpenCC(_LegacyOpenCC):
    """Convert Chinese text, customize dictionaries, and analyze it with Jieba.

    Unsupported constructor/set_config names fall back to s2t. Assignment to
    config and apply_config instead validate the name and raise ValueError.
    """

    def __init__(self, config="s2t"):
        super().__init__(self.canonicalise_config(config) if self.is_valid_config(config) else "s2t")
        self._specs = []
        self._words = []
        p, s, b, n = C.c_void_p, C.c_char_p, C.c_bool, C.c_size_t
        signatures = {
            "last_error": (p, []),
            "new_user_dict_custom": (p, [C.POINTER(_Entry), n, C.POINTER(_Spec), n]),
            "add_word": (b, [p, s, b, n, s]),
            "normalize_compat": (p, [p, s]),
            "normalize_compat_extended": (p, [p, s]),
            "detofu": (p, [p, s, C.c_uint32]),
            "keywords_pos": (C.POINTER(s), [p, s, n, s, s]),
            "keywords_and_weights_pos": (C.c_int32, [p, s, n, s, s,
                C.POINTER(n), C.POINTER(C.POINTER(s)), C.POINTER(C.POINTER(C.c_double))]),
            "free_keywords_and_weights": (None, [C.POINTER(s), C.POINTER(C.c_double), n]),
        }
        # Older bundled platform libraries may predate this additive export.
        if hasattr(self.lib, "opencc_jieba_normalize_unicode_compat"):
            signatures["normalize_unicode_compat"] = (p, [p, s])
        for name, (result, args) in signatures.items():
            fn = getattr(self.lib, "opencc_jieba_" + name)
            fn.restype, fn.argtypes = result, args
        self.lib.opencc_jieba_keywords.argtypes[2] = n
        if not self.opencc_instance:
            self._error()

    @staticmethod
    def supported_configs():
        return list(_LegacyOpenCC.config_list)

    @staticmethod
    def available_slots():
        return list(_SLOTS)

    @staticmethod
    def canonicalise_config(config):
        name = config.lower()
        if name not in _LegacyOpenCC.config_list:
            raise ValueError(f"Unsupported config: {config}")
        return name

    @staticmethod
    def is_valid_config(config):
        return isinstance(config, str) and config.lower() in _LegacyOpenCC.config_list

    @property
    def config(self):
        return self._config

    @config.setter
    def config(self, value):
        self._config = self.canonicalise_config(value)

    def apply_config(self, config):
        self.config = config

    def set_config(self, config):
        self.config = config if self.is_valid_config(config) else "s2t"

    def _error(self):
        ptr = self.lib.opencc_jieba_last_error()
        message = "Native OpenCC operation failed"
        if ptr:
            try:
                message = C.string_at(ptr).decode("utf-8")
            finally:
                self.lib.opencc_jieba_free_string(ptr)
        raise ValueError(message)

    def _string(self, name, *args):
        ptr = getattr(self.lib, "opencc_jieba_" + name)(*args)
        if not ptr:
            self._error()
        try:
            return C.string_at(ptr).decode("utf-8")
        finally:
            self.lib.opencc_jieba_free_string(ptr)

    def _array(self, name, *args):
        ptr = getattr(self.lib, "opencc_jieba_" + name)(*args)
        if not ptr:
            self._error()
        try:
            result, i = [], 0
            while ptr[i] is not None:
                result.append(ptr[i].decode("utf-8"))
                i += 1
            return result
        finally:
            self.lib.opencc_jieba_free_string_array(ptr)

    def convert(self, input_text, punctuation=False):
        return self._string("convert", self.opencc_instance, _utf8(input_text), _utf8(self.config), punctuation)

    def zho_check(self, input_text):
        result = self.lib.opencc_jieba_zho_check(self.opencc_instance, _utf8(input_text))
        if result < 0:
            self._error()
        return result

    def normalize_compat(self, text):
        return self._string("normalize_compat", self.opencc_instance, _utf8(text))

    def normalize_compat_extended(self, text):
        return self._string("normalize_compat_extended", self.opencc_instance, _utf8(text))

    def normalize_unicode_compat(self, text):
        """Apply curated Unicode mappings, preserving CJK compatibility ideographs.

        Requires a native library exporting opencc_jieba_normalize_unicode_compat.
        """
        if not hasattr(self.lib, "opencc_jieba_normalize_unicode_compat"):
            raise RuntimeError(
                "The native library lacks opencc_jieba_normalize_unicode_compat; "
                "rebuild or update the bundled native library."
            )
        return self._string("normalize_unicode_compat", self.opencc_instance, _utf8(text))

    @staticmethod
    def _level(level):
        key = level.strip().lower()
        if key == "all":
            return 0
        if key.startswith("ext-"):
            key = key[4:]
        elif key.startswith("ext"):
            key = key[3:]
        if key not in list("bcdefghi"):
            raise ValueError(f"Unsupported DeTofu level: {level}")
        return "bcdefghi".index(key)

    def detofu(self, text, level="ExtB"):
        return self._string("detofu", self.opencc_instance, _utf8(text), self._level(level))

    def detofu_with_custom_file(self, text, level, path):
        # The C ABI exposes built-ins only. Match Rust's single-pass custom
        # precedence without feeding custom replacement characters back into it.
        threshold, mappings = self._level(level), {}
        for number, line in enumerate(Path(path).read_text(encoding="utf-8").splitlines(), 1):
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = [part.strip() for part in line.split("\t")]
            if len(parts) < 3 or not parts[0] or not parts[1]:
                raise ValueError(f"{path}:{number}: expected character<TAB>fallback<TAB>extension")
            if self._level(parts[2]) >= threshold:
                mappings[parts[0][0]] = parts[1][0]
        chunks, start = [], 0
        for i, char in enumerate(text):
            if char in mappings:
                chunks.extend((self.detofu(text[start:i], level), mappings[char]))
                start = i + 1
        chunks.append(self.detofu(text[start:], level))
        return "".join(chunks)

    def jieba_cut(self, input_text, hmm=True):
        return self._array("cut", self.opencc_instance, _utf8(input_text), hmm)

    def jieba_cut_for_search(self, input_text, hmm=True):
        return self._array("cut_for_search", self.opencc_instance, _utf8(input_text), hmm)

    def jieba_cut_all(self, input_text):
        return self._array("cut_all", self.opencc_instance, _utf8(input_text))

    def jieba_tag(self, input_text, hmm=True):
        ptr = self.lib.opencc_jieba_tag(self.opencc_instance, _utf8(input_text), hmm)
        if not ptr:
            self._error()
        try:
            result, i = [], 0
            while ptr[i].word is not None:
                result.append((ptr[i].word.decode("utf-8"), ptr[i].tag.decode("utf-8")))
                i += 1
            return result
        finally:
            self.lib.opencc_jieba_free_tag_array(ptr)

    def jieba_cut_and_join(self, input_text, delimiter="/", *legacy, hmm=True):
        # Retain the old (text, hmm, delimiter) positional calling convention.
        if isinstance(delimiter, bool):
            hmm, delimiter = delimiter, legacy[0] if legacy else ", "
        elif legacy:
            raise TypeError("Unexpected positional arguments")
        return self._string("cut_and_join", self.opencc_instance, _utf8(input_text), hmm, _utf8(delimiter))

    def jieba_segment_join(self, input_text, mode="cut", delim=" ", hmm=True, separator="/"):
        mode = mode.lower()
        if mode == "tag":
            return delim.join(f"{w}{separator}{t}" for w, t in self.jieba_tag(input_text, hmm))
        if mode == "full":
            return delim.join(self.jieba_cut_all(input_text))
        if mode in ("cut", "search"):
            method = self.jieba_cut if mode == "cut" else self.jieba_cut_for_search
            return delim.join(method(input_text, hmm))
        raise ValueError(f"Unsupported segmentation mode: {mode}")

    def jieba_join_str(self, strings, delimiter=" "):
        encoded = [_utf8(s) for s in strings]
        array = (C.c_char_p * (len(encoded) + 1))(*encoded, None)
        return self._string("join_str", array, _utf8(delimiter))

    def _keywords(self, text, top_k, allowed_pos, method, weighted=False):
        args = (self.opencc_instance, _utf8(text), _frequency(top_k),
                _utf8(method), _utf8(" ".join(allowed_pos)) if allowed_pos else None)
        if not weighted:
            return self._array("keywords_pos", *args)
        count, words, weights = C.c_size_t(), C.POINTER(C.c_char_p)(), C.POINTER(C.c_double)()
        status = self.lib.opencc_jieba_keywords_and_weights_pos(
            *args, C.byref(count), C.byref(words), C.byref(weights))
        if status < 0:
            self._error()
        try:
            return [(words[i].decode("utf-8"), weights[i]) for i in range(count.value)]
        finally:
            self.lib.opencc_jieba_free_keywords_and_weights(words, weights, count)

    def jieba_keyword_extract_textrank(self, input_text, top_k=10, allowed_pos=None):
        return self._keywords(input_text, top_k, allowed_pos, "textrank")

    def jieba_keyword_extract_tfidf(self, input_text, top_k=10, allowed_pos=None):
        return self._keywords(input_text, top_k, allowed_pos, "tfidf")

    def jieba_keyword_weight_textrank(self, input_text, top_k=10, allowed_pos=None):
        return self._keywords(input_text, top_k, allowed_pos, "textrank", True)

    def jieba_keyword_weight_tfidf(self, input_text, top_k=10, allowed_pos=None):
        return self._keywords(input_text, top_k, allowed_pos, "tfidf", True)

    def _rebuild(self, specs, words):
        """Publish a new native instance only after construction/replay succeeds."""
        arrays = [(_Pair * len(pairs))(*[_Pair(_utf8(a), _utf8(b)) for a, b in pairs])
                  for _, _, pairs in specs]
        ffi_specs = (_Spec * len(specs))(*[
            _Spec(slot, mode, pairs, len(pairs))
            for (slot, mode, _), pairs in zip(specs, arrays)])
        instance = self.lib.opencc_jieba_new_user_dict_custom(None, 0, ffi_specs, len(specs))
        if not instance:
            self._error()
        try:
            for word, freq, tag in words:
                if not self.lib.opencc_jieba_add_word(instance, word, freq is not None, freq or 0, tag):
                    self._error()
        except BaseException:
            self.lib.opencc_jieba_delete(instance)
            raise
        old, self.opencc_instance = self.opencc_instance, instance
        self._specs, self._words = specs, words
        self.lib.opencc_jieba_delete(old)

    def load_custom_dicts(self, specs):
        prepared = []
        aliases = {"JPShinjitaiCharacters": "JPSCharacters", "JPShinjitaiCharactersRev": "JPSCharactersRev",
                   "JPShinjitaiPhrases": "JPSPhrases"}
        for spec in specs:
            slot = spec["slot"].strip()
            if slot.lower().endswith(".txt"):
                slot = slot[:-4]
            names = {name.lower(): name for name in _SLOTS}
            names.update({name.lower(): value for name, value in aliases.items()})
            slot = names.get(slot.lower(), slot)
            mode = spec.get("mode")
            mode = ("append" if mode is None else mode).strip().lower()
            if slot not in _SLOTS or mode not in ("append", "override"):
                raise ValueError(f"Invalid dictionary slot or mode: {slot}:{mode}")
            pairs = [tuple(pair) for pair in spec["pairs"]]
            for source, target in pairs:
                _utf8(source), _utf8(target)
            prepared.append((_SLOTS.index(slot) + 1, 1 if mode == "append" else 2, pairs))
        if prepared:
            self._rebuild(self._specs + prepared, list(self._words))

    def load_custom_dict_files(self, specs):
        prepared = []
        for spec in specs:
            pairs = []
            for path in spec["files"]:
                for number, line in enumerate(Path(path).read_text(encoding="utf-8-sig").splitlines(), 1):
                    line = line.rstrip()
                    if not line or line.lstrip().startswith("#"):
                        continue
                    source, sep, target = line.partition("\t")
                    targets = target.split()
                    if not sep or not source or not targets:
                        raise ValueError(f"{path}:{number}: expected source<TAB>target")
                    pairs.append((source, targets[0]))
            prepared.append({"slot": spec["slot"], "mode": spec.get("mode", "append"), "pairs": pairs})
        self.load_custom_dicts(prepared)

    @staticmethod
    def _word(word, freq, tag):
        encoded, encoded_tag = _utf8(word), None if tag is None else _utf8(tag)
        if not word or any(c.isspace() for c in word) or (tag is not None and any(c.isspace() for c in tag)):
            raise ValueError("Word must be nonempty; word and tag must not contain whitespace")
        return encoded, None if freq is None else _frequency(freq), encoded_tag

    def add_word(self, word, freq=None, tag=None):
        entry = self._word(word, freq, tag)
        if not self.lib.opencc_jieba_add_word(self.opencc_instance, entry[0], freq is not None, freq or 0, entry[2]):
            self._error()
        self._words.append(entry)

    def load_user_dict_entries(self, entries):
        prepared = [self._word(e["word"], _frequency(e["freq"]), e.get("tag")) for e in entries]
        if prepared:
            self._rebuild(list(self._specs), self._words + prepared)

    def load_user_dict_files(self, files):
        entries = []
        for path in files:
            for number, line in enumerate(Path(path).read_text(encoding="utf-8").splitlines(), 1):
                parts = line.split()
                if not parts:
                    continue
                if len(parts) not in (2, 3) or not parts[1].isascii() or not parts[1].lstrip("+").isdigit():
                    raise ValueError(f"{path}:{number}: expected word freq [tag]")
                entries.append({"word": parts[0], "freq": int(parts[1]), **({"tag": parts[2]} if len(parts) == 3 else {})})
        self.load_user_dict_entries(entries)

    def load_user_dict(self, path):
        self.load_user_dict_files([path])

    @classmethod
    def from_dicts(cls, config="s2t", specs=None):
        cc = cls(config)
        cc.load_custom_dicts(specs or [])
        return cc

    @classmethod
    def from_dict_files(cls, config="s2t", specs=None):
        cc = cls(config)
        cc.load_custom_dict_files(specs or [])
        return cc

    @classmethod
    def from_user_dict_entries(cls, config="s2t", entries=None):
        cc = cls(config)
        cc.load_user_dict_entries(entries or [])
        return cc

    @classmethod
    def from_user_dict_files(cls, config="s2t", files=None):
        cc = cls(config)
        cc.load_user_dict_files(files or [])
        return cc


__all__ = ["OpenCC", "CustomDictSpec", "CustomDictFileSpec", "UserDictEntry"]
