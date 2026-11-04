"""Command-line conversion, segmentation, and Office/EPUB conversion.

Text commands read a file or stdin and write a file or stdout. Diagnostics go
to stderr so redirected output contains only the requested text.
"""

import argparse
import codecs
import io
import os
import sys
from pathlib import Path
from typing import Callable, List, Optional, Tuple

from opencc_jieba_rs import CustomDictFileSpec, OpenCC

SUPPORTED_CONFIGS = OpenCC.supported_configs()
AVAILABLE_SLOTS = OpenCC.available_slots()
CONFIG_HELP = "Configuration: " + "|".join(SUPPORTED_CONFIGS)
SLOT_HELP = "Available slots: " + "|".join(AVAILABLE_SLOTS)

TextConverter = Callable[[str], str]


def resolve_config(config: Optional[str]) -> Optional[str]:
    if config is None:
        print("ℹ️  Config not set. Use default: s2t", file=sys.stderr)
        return "s2t"

    try:
        return OpenCC.canonicalise_config(config)
    except ValueError:
        print(f"❌ Invalid OpenCC config: {config}", file=sys.stderr)
        print(
            f"   Supported configs: {' | '.join(SUPPORTED_CONFIGS)}",
            file=sys.stderr,
        )
        return None


def resolve_slot(slot: str) -> str:
    slot_key = slot.strip().casefold()

    for available_slot in AVAILABLE_SLOTS:
        if available_slot.casefold() == slot_key:
            return available_slot

    raise ValueError(
        f"Invalid custom dictionary slot: {slot}. "
        f"Expected one of: {' | '.join(AVAILABLE_SLOTS)}"
    )


def parse_custom_dict_spec(spec: str) -> CustomDictFileSpec:
    """
    Parse one ``--custom-dict`` value using ``slot:mode:path`` syntax.

    Splitting is limited to two separators so Windows drive-letter paths such
    as ``R:\\dicts\\UserDict.txt`` remain intact.
    """
    parts = spec.split(":", 2)
    if len(parts) != 3:
        raise ValueError(
            "Invalid --custom-dict spec {!r}. Expected slot:mode:path".format(spec)
        )

    slot, mode, path = (part.strip() for part in parts)

    if not slot:
        raise ValueError(
            "Invalid --custom-dict spec {!r}: slot is empty".format(spec)
        )

    mode = mode.lower()
    if mode not in ("append", "override"):
        raise ValueError(
            "Invalid --custom-dict mode {!r}. Expected append or override".format(
                mode
            )
        )

    if not path:
        raise ValueError(
            "Invalid --custom-dict spec {!r}: path is empty".format(spec)
        )

    if not Path(path).is_file():
        raise ValueError("Custom dictionary file not found: {}".format(path))

    return {
        "slot": resolve_slot(slot),
        "mode": mode,
        "files": [path],
    }


def custom_dict_specs_from_args(args: argparse.Namespace) -> List[CustomDictFileSpec]:
    return [
        parse_custom_dict_spec(spec)
        for spec in (getattr(args, "custom_dict", None) or [])
    ]


def user_dict_files_from_args(args: argparse.Namespace) -> List[str]:
    """Validate and return ``-U/--user-dict-file`` paths in CLI order."""
    paths = getattr(args, "user_dict_file", None) or []

    for path in paths:
        if not Path(path).is_file():
            raise ValueError("Jieba user dictionary file not found: {}".format(path))

    return paths


def build_opencc(
        config: str,
        args: argparse.Namespace,
) -> Tuple[OpenCC, List[CustomDictFileSpec]]:
    """
    Construct one OpenCC instance and apply optional Jieba and OpenCC files.

    Jieba user dictionaries are loaded first so custom OpenCC phrase mappings
    see the intended segmentation.
    """
    custom_specs = custom_dict_specs_from_args(args)
    user_files = user_dict_files_from_args(args)

    opencc = OpenCC(config)

    if user_files:
        opencc.load_user_dict_files(user_files)

    if custom_specs:
        opencc.load_custom_dict_files(custom_specs)

    return opencc, custom_specs


def build_jieba(args: argparse.Namespace) -> OpenCC:
    """Construct the Jieba instance and load only ``-U`` user dictionaries."""
    user_files = user_dict_files_from_args(args)
    opencc = OpenCC("s2t")

    if user_files:
        opencc.load_user_dict_files(user_files)

    return opencc


def paths_refer_to_same_file(input_path: str, output_path: str) -> bool:
    """Recognize identical paths and existing symbolic/hard-link aliases."""
    if os.path.normcase(os.path.abspath(input_path)) == os.path.normcase(
            os.path.abspath(output_path)
    ):
        return True
    try:
        return os.path.samefile(input_path, output_path)
    except OSError:
        # A new output file has no filesystem identity yet.
        return False


def validate_input_output_paths(
        input_path: Optional[str],
        output_path: Optional[str],
) -> bool:
    if input_path and not Path(input_path).is_file():
        print(f"❌ Input file not found: {input_path}", file=sys.stderr)
        return False

    if input_path and output_path and paths_refer_to_same_file(input_path, output_path):
        print("❌ Input and output files must be different.", file=sys.stderr)
        return False

    return True


def read_text_input(input_path: Optional[str], encoding: str) -> Optional[str]:
    try:
        if input_path:
            with io.open(input_path, "r", encoding=encoding) as stream:
                return stream.read()

        binary_stream = getattr(sys.stdin, "buffer", None)
        if binary_stream is not None:
            return binary_stream.read().decode(encoding)
        # Embedded callers may supply an already-decoded text stream.
        codecs.lookup(encoding)
        return sys.stdin.read()
    except LookupError as ex:
        print(f"❌ Invalid input encoding '{encoding}': {ex}", file=sys.stderr)
    except (OSError, UnicodeError) as ex:
        source = input_path or "<stdin>"
        print(f"❌ Failed to read input '{source}': {ex}", file=sys.stderr)

    return None


def write_text_output(
        output_str: str,
        output_path: Optional[str],
        encoding: str,
) -> bool:
    # Validate even for interactive consoles, which write Unicode directly.
    try:
        codecs.lookup(encoding)
    except LookupError as ex:
        print(f"❌ Invalid output encoding '{encoding}': {ex}", file=sys.stderr)
        return False

    try:
        if output_path:
            with io.open(output_path, "w", encoding=encoding) as stream:
                stream.write(output_str)
        elif sys.stdout.isatty() or not hasattr(sys.stdout, "buffer"):
            sys.stdout.write(output_str)
            sys.stdout.flush()
        else:
            sys.stdout.buffer.write(output_str.encode(encoding))
            sys.stdout.buffer.flush()
    except (OSError, UnicodeError) as ex:
        target = output_path or "<stdout>"
        print(f"❌ Failed to write output '{target}': {ex}", file=sys.stderr)
        return False

    return True


def normalize_input(opencc: OpenCC, input_str: str, args: argparse.Namespace) -> str:
    if getattr(args, "norm_compat_extended", False):
        return opencc.normalize_compat_extended(input_str)
    if getattr(args, "norm_compat", False):
        return opencc.normalize_compat(input_str)
    return input_str


def validate_detofu_args(args: argparse.Namespace) -> bool:
    """Validate shared DeTofu CLI argument dependencies."""
    if getattr(args, "detofu_file", None) and not getattr(args, "detofu", None):
        print("❌ --detofu-file requires --detofu", file=sys.stderr)
        return False
    return True


def make_text_converter(
        opencc: OpenCC,
        args: argparse.Namespace,
) -> TextConverter:
    """
    Build the shared text transformation pipeline.

    The caller owns argument validation, OpenCC construction, dictionaries,
    I/O, and command-specific behavior.

    Pipeline:
        Normalize -> Convert -> DeTofu
    """
    normalize: Optional[TextConverter] = None
    if getattr(args, "norm_compat_extended", False):
        normalize = opencc.normalize_compat_extended
    elif getattr(args, "norm_compat", False):
        normalize = opencc.normalize_compat
    convert = opencc.convert
    punct = getattr(args, "punct", False)

    detofu: Optional[str] = getattr(args, "detofu", None)
    detofu_file: Optional[str] = getattr(args, "detofu_file", None)
    # Resolve bound methods once: Office documents call this pipeline for many
    # separate text fragments. Custom DeTofu files still use the native API.
    builtin_fallback = opencc.detofu
    custom_fallback = opencc.detofu_with_custom_file

    def convert_text(text: str) -> str:
        if normalize is not None:
            text = normalize(text)

        text = convert(text, punct)

        if detofu is not None:
            if detofu_file is not None:
                text = custom_fallback(text, detofu, detofu_file)
            else:
                text = builtin_fallback(text, detofu)

        return text

    return convert_text


def normalization_status(args: argparse.Namespace) -> str:
    if getattr(args, "norm_compat_extended", False):
        return ", norm-compat:extended"
    if getattr(args, "norm_compat", False):
        return ", norm-compat"
    return ""


def custom_dict_status(specs: List[CustomDictFileSpec]) -> str:
    if not specs:
        return ""

    details = ",".join(
        f"{spec['slot']}:{spec.get('mode', 'append')}" for spec in specs
    )
    return f", custom:{details}"


def finish_text_status(
        output_str: str,
        output_path: Optional[str],
        status: str,
) -> None:
    if not sys.stderr.isatty():
        return

    # Add only a display newline after direct interactive stdout output.
    if not output_path and sys.stdout.isatty() and output_str and not output_str.endswith("\n"):
        sys.stdout.write("\n")
        sys.stdout.flush()

    print(status, file=sys.stderr)


def subcommand_convert(args: argparse.Namespace) -> int:
    config = resolve_config(args.config)
    if config is None:
        return 1
    args.config = config

    if not validate_input_output_paths(args.input, args.output):
        return 1

    if not validate_detofu_args(args):
        return 1

    try:
        opencc, specs = build_opencc(config, args)
    except (OSError, RuntimeError, ValueError) as ex:
        print(f"❌ Failed to initialize OpenCC: {ex}", file=sys.stderr)
        return 1

    if args.input is None and sys.stdin.isatty():
        print(
            "Input text to convert, <Ctrl+Z> (Windows) or <Ctrl+D> (Unix) "
            "then Enter to submit:",
            file=sys.stderr,
        )

    input_str = read_text_input(args.input, args.in_enc)
    if input_str is None:
        return 1

    try:
        text_converter = make_text_converter(opencc, args)
        output_str = text_converter(input_str)
    except (OSError, RuntimeError, ValueError) as ex:
        print(f"❌ Conversion failed: {ex}", file=sys.stderr)
        return 1

    if not write_text_output(output_str, args.output, args.out_enc):
        return 1

    in_from = args.input or "<stdin>"
    out_to = args.output or "<stdout>"
    status = (
        f"Conversion completed ({config}"
        f"{normalization_status(args)}"
        f"{custom_dict_status(specs)}"
        f"): {in_from} -> {out_to}"
    )
    finish_text_status(output_str, args.output, status)
    return 0


def subcommand_segment(args: argparse.Namespace) -> int:
    if not validate_input_output_paths(args.input, args.output):
        return 1

    try:
        # Segmentation uses only Jieba user dictionaries (-U).
        # OpenCC custom dictionaries (-D) are intentionally not loaded here.
        opencc = build_jieba(args)
    except (OSError, RuntimeError, ValueError) as ex:
        print(f"❌ Failed to initialize OpenCC/Jieba: {ex}", file=sys.stderr)
        return 1

    if args.input is None and sys.stdin.isatty():
        print(
            "Input text to segment, <Ctrl+Z> (Windows) or <Ctrl+D> (Unix) "
            "then Enter to submit:",
            file=sys.stderr,
        )

    input_str = read_text_input(args.input, args.in_enc)
    if input_str is None:
        return 1

    try:
        input_str = normalize_input(opencc, input_str, args)

        mode = args.mode
        delim = args.delim if args.delim is not None else " "
        separator = args.separator if args.separator is not None else "/"
        hmm = not args.no_hmm
        output_str = opencc.jieba_segment_join(
            input_str, mode=mode, delim=delim, hmm=hmm, separator=separator
        )
    except ValueError as ex:
        print(f"❌ Segmentation failed: {ex}", file=sys.stderr)
        return 1

    if not write_text_output(output_str, args.output, args.out_enc):
        return 1

    in_from = args.input or "<stdin>"
    out_to = args.output or "<stdout>"
    hmm_status = "" if mode == "full" else f", HMM:{hmm}"
    status = (
        f"Segmentation completed ({mode}"
        f"{hmm_status}"
        f"{normalization_status(args)}"
        f"): {in_from} -> {out_to}"
    )
    finish_text_status(output_str, args.output, status)
    return 0


def subcommand_office(args: argparse.Namespace) -> int:
    """Convert an Office or EPUB document using OpenCC + Jieba.

    Reuse the shared text pipeline for document contents and, optionally,
    for the automatically generated output filename.

    Jieba user dictionaries (-U) are loaded before OpenCC custom
    dictionaries (-D). Explicit output filenames are never converted.
    """
    from .office_helper import OFFICE_FORMATS, convert_office_doc

    config = resolve_config(args.config)
    if config is None:
        return 1

    input_file = args.input
    output_file = args.output
    office_format = args.format.lower() if args.format else None
    keep_font = args.keep_font
    convert_filename = getattr(args, "convert_filename", False)

    if not input_file:
        print("❌ Input file is missing.", file=sys.stderr)
        return 1

    input_path = Path(input_file)
    if not input_path.is_file():
        print(f"❌ Input file not found: {input_file}", file=sys.stderr)
        return 1

    if office_format is None:
        office_format = input_path.suffix.lower().lstrip(".")

    if office_format not in OFFICE_FORMATS:
        print(
            f"❌ Unsupported Office format: {office_format or '(none)'}",
            file=sys.stderr,
        )
        print(
            f"   Supported formats: {', '.join(OFFICE_FORMATS)}",
            file=sys.stderr,
        )
        return 1

    if not validate_detofu_args(args):
        return 1

    # Reuse the dictionary-loading SSOT:
    # Jieba -U first, then OpenCC -D.
    try:
        opencc, _ = build_opencc(config, args)
    except (OSError, RuntimeError, ValueError) as ex:
        print(
            f"❌ Failed to initialize OpenCC: {ex}",
            file=sys.stderr,
        )
        return 1

    office_text_converter = make_text_converter(opencc, args)

    # Generate the default output filename only when -o is absent.
    if not output_file:
        stem = input_path.stem

        if convert_filename:
            try:
                stem = office_text_converter(stem)
            except (OSError, RuntimeError, ValueError) as ex:
                print(
                    f"❌ Filename conversion failed: {ex}",
                    file=sys.stderr,
                )
                return 1

            if not stem or stem in (".", ".."):
                print(
                    "❌ Filename conversion produced an invalid name.",
                    file=sys.stderr,
                )
                return 1

            # A converted stem must remain one filename component.
            if (
                    "/" in stem
                    or "\\" in stem
                    or "\x00" in stem
            ):
                print(
                    "❌ Converted filename contains a path separator "
                    "or NUL character.",
                    file=sys.stderr,
                )
                return 1

        output_file = str(
            input_path.with_name(f"{stem}_converted.{office_format}")
        )
        print(
            f"ℹ️ Output file not specified. Using: {output_file}",
            file=sys.stderr,
        )

    # Append the format extension when an explicit output has none.
    elif not Path(output_file).suffix:
        output_file += f".{office_format}"
        print(
            f"ℹ️ Auto-extension applied: {output_file}",
            file=sys.stderr,
        )

    # Never overwrite the input package.
    if paths_refer_to_same_file(input_file, output_file):
        print(
            "❌ Input and output files must be different.",
            file=sys.stderr,
        )
        return 1

    try:
        success, message = convert_office_doc(
            input_path=str(input_path),
            output_path=output_file,
            office_format=office_format,
            office_text_converter=office_text_converter,
            keep_font=keep_font,
        )
    except Exception as ex:
        print(
            f"❌ Error during Office document conversion: {ex}",
            file=sys.stderr,
        )
        return 1

    if not success:
        print(
            f"❌ Office document conversion failed: {message}",
            file=sys.stderr,
        )
        return 1

    print(
        f"{message} ({config})\n"
        f"📁 Output saved to: {os.path.abspath(output_file)}",
        file=sys.stderr,
    )
    return 0


def add_custom_dict_option(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "-D",
        "--custom-dict",
        action="append",
        metavar="<slot:mode:path>",
        help=(
                "Load custom OpenCC dictionary file. "
                "Format: slot:mode:path, e.g. STPhrases:append:custom.txt. "
                "Can be used multiple times. " + SLOT_HELP
        ),
    )


def add_user_dict_option(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "-U",
        "--user-dict-file",
        action="append",
        metavar="<file>",
        help=(
            "Load Jieba user dictionary file using 'word freq [tag]' format. "
            "Can be used multiple times."
        ),
    )


def add_conversion_dictionary_options(parser: argparse.ArgumentParser) -> None:
    add_custom_dict_option(parser)
    add_user_dict_option(parser)


def add_normalization_options(parser: argparse.ArgumentParser, action: str) -> None:
    parser.add_argument(
        "-n",
        "--norm-compat",
        action="store_true",
        help=f"Normalize CJK Compatibility Ideographs before {action}.",
    )
    parser.add_argument(
        "-E",
        "--norm-compat-extended",
        action="store_true",
        help=f"Normalize extended Unicode compatibility forms before {action}.",
    )


def add_detofu_options(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--detofu",
        nargs="?",
        const="all",
        default=None,
        metavar="<level>",
        help=(
            "Apply DeTofu after conversion. "
            "Optional level: all/ExtB, ExtC, ExtD, ExtE, ExtF, ExtG, ExtH, ExtI."
        ),
    )
    parser.add_argument(
        "--detofu-file",
        metavar="<file>",
        help="Load additional DeTofu mappings from a UTF-8 text file. Requires --detofu.",
    )


def add_text_io_options(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "-i", "--input", metavar="<file>", help="Read input text from <file>."
    )
    parser.add_argument(
        "-o", "--output", metavar="<file>", help="Write output text to <file>."
    )
    parser.add_argument(
        "--in-enc",
        metavar="<encoding>",
        default="UTF-8",
        help="Encoding for input.",
    )
    parser.add_argument(
        "--out-enc",
        metavar="<encoding>",
        default="UTF-8",
        help="Encoding for output.",
    )


def build_parser() -> argparse.ArgumentParser:
    """Build the CLI parser without consuming process arguments or running I/O."""
    parser = argparse.ArgumentParser(
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
        description="opencc_jieba_rs – OpenCC + Jieba CLI",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    parser_convert = subparsers.add_parser(
        "convert",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
        help="Convert Chinese text using OpenCC + Jieba",
    )
    add_text_io_options(parser_convert)
    parser_convert.add_argument(
        "-c", "--config", metavar="<conversion>", help=CONFIG_HELP
    )
    parser_convert.add_argument(
        "-p",
        "--punct",
        action="store_true",
        help="Enable punctuation conversion.",
    )
    add_normalization_options(parser_convert, "conversion")
    add_detofu_options(parser_convert)
    add_conversion_dictionary_options(parser_convert)
    parser_convert.set_defaults(func=subcommand_convert)

    parser_segment = subparsers.add_parser(
        "segment",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
        help="Segment Chinese text using Jieba",
    )
    add_text_io_options(parser_segment)
    parser_segment.add_argument(
        "-d",
        "--delim",
        metavar="<char>",
        default=" ",
        help="Text between tokens; use an empty string to concatenate tokens.",
    )
    parser_segment.add_argument(
        "-s",
        "--separator",
        metavar="<char>",
        default="/",
        help="Text between each word and its POS tag; used only in tag mode.",
    )
    parser_segment.add_argument(
        "--no-hmm",
        action="store_true",
        help="Disable HMM word discovery (ignored in full mode).",
    )
    parser_segment.add_argument(
        "-m",
        "--mode",
        choices=["cut", "search", "full", "tag"],
        default="cut",
        help=(
            "cut: accurate words; search: overlapping tokens for indexing; "
            "full: dictionary matches without disambiguation; tag: word/POS pairs."
        ),
    )
    add_normalization_options(parser_segment, "segmentation")
    add_user_dict_option(parser_segment)
    # Backward compatibility: accept -D/--custom-dict for segment, but do not
    # expose or load it. OpenCC custom dictionaries do not affect Jieba-only
    # segmentation.
    parser_segment.add_argument(
        "-D",
        "--custom-dict",
        action="append",
        metavar="<slot:mode:path>",
        help=argparse.SUPPRESS,
    )
    parser_segment.set_defaults(func=subcommand_segment)

    parser_office = subparsers.add_parser(
        "office",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
        help="Convert Office document Chinese text using OpenCC + Jieba",
    )
    parser_office.add_argument(
        "-i",
        "--input",
        metavar="<file>",
        required=True,
        help="Input Office or EPUB document.",
    )
    parser_office.add_argument(
        "-o",
        "--output",
        metavar="<file>",
        help=(
            "Output document path. Defaults to "
            "<input-stem>_converted.<format>. "
            "A missing extension is appended automatically."
        ),
    )
    parser_office.add_argument(
        "-c",
        "--config",
        metavar="<conversion>",
        help=CONFIG_HELP,
    )
    parser_office.add_argument(
        "-p",
        "--punct",
        action="store_true",
        help="Enable punctuation conversion.",
    )
    parser_office.add_argument(
        "-F",
        "--convert-filename",
        action="store_true",
        help=(
            "Convert the input filename stem when generating the default "
            "output filename. Ignored when --output is specified."
        ),
    )
    parser_office.add_argument(
        "-f",
        "--format",
        choices=("docx", "xlsx", "pptx", "odt", "ods", "odp", "epub"),
        help="Office or EPUB format (inferred from input extension by default).",
    )
    parser_office.add_argument(
        "-k", "--keep-font",
        action="store_true",
        help="Preserve font-family information in Office content.",
    )
    add_normalization_options(parser_office, "conversion")
    add_detofu_options(parser_office)
    # Reuse the existing -D and -U dictionary options.
    add_conversion_dictionary_options(parser_office)

    parser_office.set_defaults(func=subcommand_office)

    return parser


def main(argv: Optional[List[str]] = None) -> int:
    """Run a command and return its exit status; None reads ``sys.argv[1:]``."""
    args = build_parser().parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
