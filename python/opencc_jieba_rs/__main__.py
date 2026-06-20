from __future__ import print_function

import argparse
import sys
import io
from opencc_jieba_rs import OpenCC


def subcommand_convert(args):
    if args.config is None:
        print("Please set conversion configuration.", file=sys.stderr)
        return 1

    opencc = OpenCC(args.config)

    with io.open(args.input if args.input else 0, encoding=args.in_enc) as f:
        input_str = f.read()

    output_str = opencc.convert(input_str, args.punct)

    with io.open(args.output if args.output else 1, 'w', encoding=args.out_enc) as f:
        f.write(output_str)

    in_from = args.input if args.input else "<stdin>"
    out_to = args.output if args.output else "<stdout>"
    if sys.stderr.isatty():
        if output_str and not output_str.endswith("\n"):
            print()
        print(f"Conversion completed ({args.config}): {in_from} -> {out_to}", file=sys.stderr)
    return 0


def subcommand_office(args):
    import os
    from pathlib import Path
    from .office_helper import OFFICE_FORMATS, convert_office_doc

    if args.config is None:
        print("ℹ️  Config not specified. Use default 's2t'", file=sys.stderr)
    args.config = 's2t'

    input_file = args.input
    output_file = args.output
    office_format = args.format.lower() if args.format else None
    config = args.config
    punct = args.punct
    keep_font = getattr(args, "keep_font", False)

    # Check for missing input/output files
    if not input_file and not output_file:
        print("❌  Input and output files are missing.", file=sys.stderr)
        return 1
    if not input_file:
        print("❌  Input file is missing.", file=sys.stderr)
        return 1
    if not Path(input_file).is_file():
        print(f"❌ Input file not found: {input_file}", file=sys.stderr)
        return 1

    # Determine office format from file extension if not provided
    if office_format:
        if office_format not in OFFICE_FORMATS:
            print(f"❌  Unsupported Office format: {args.format}", file=sys.stderr)
            return 1
    else:
        file_ext = os.path.splitext(input_file)[1].lower().lstrip(".")
        if file_ext not in OFFICE_FORMATS:
            print(f"❌  Invalid Office file extension: .{file_ext or '(none)'}", file=sys.stderr)
            print("   Valid extensions: .docx | .xlsx | .pptx | .odt | .ods | .odp | .epub", file=sys.stderr)
            return 1
        office_format = str(file_ext)

    # If output file is not specified, generate one based on input file
    if not output_file:
        input_path = Path(input_file)
        input_dir = input_path.parent if input_path.parent != Path("") else Path.cwd()
        output_path = input_dir / f"{input_path.stem}_converted.{office_format}"
        output_file = str(output_path)
        print(f"ℹ️  Output file not specified. Using: {output_path}", file=sys.stderr)

    elif not os.path.splitext(output_file)[1]:
        output_file += f".{office_format}"
        print(f"ℹ️  Auto-extension applied: {output_file}", file=sys.stderr)

    try:
        # Perform Office document conversion
        success, message = convert_office_doc(
            input_file,
            output_file,
            office_format,
            OpenCC(config),
            punct,
            keep_font,
        )
        if success:
            print(f"{message}\n📁  Output saved to: {os.path.abspath(output_file)}", file=sys.stderr)
            return 0
        else:
            print(f"❌  Conversion failed: {message}", file=sys.stderr)
            return 1
    except Exception as ex:
        print(f"❌  Error during Office document conversion: {str(ex)}", file=sys.stderr)
        return 1


def subcommand_segment(args):
    import io
    opencc = OpenCC()  # Default config if not needed for segmentation

    # Prompt only if reading from stdin, and it's interactive (i.e., not piped or redirected)
    if args.input is None and sys.stdin.isatty():
        print(
            "Input text to segment, <Ctrl+Z> (Windows) or <Ctrl+D> (Unix) then Enter to submit:",
            file=sys.stderr
        )

    with io.open(args.input if args.input else 0, encoding=args.in_enc) as f:
        input_str = f.read()

    mode = args.mode
    delim = args.delim if args.delim not in (None, "", "/") else " "
    separator = args.separator if args.separator not in (None, "") else "/"
    hmm = not args.no_hmm

    if mode == "cut":
        segments = opencc.jieba_cut(input_str, hmm)
        output_str = delim.join(segments)

    elif mode == "search":
        segments = opencc.jieba_cut_for_search(input_str, hmm)
        output_str = delim.join(segments)

    elif mode == "full":
        # Prefer explicit full-mode API if your binding exposes one
        if hasattr(opencc, "jieba_cut_all"):
            segments = opencc.jieba_cut_all(input_str)
        elif hasattr(opencc, "jieba_cut_full"):
            segments = opencc.jieba_cut_full(input_str)
        else:
            print("❌  Full mode is not available in this build of opencc_jieba_pyo3.", file=sys.stderr)
            return 1
        output_str = delim.join(segments)

    elif mode == "tag":
        tagged = opencc.jieba_tag(input_str, hmm)
        output_str = delim.join(f"{word}{separator}{tag}" for word, tag in tagged)

    else:
        print(f"❌  Invalid segmentation mode: {mode}", file=sys.stderr)
        return 1

    with io.open(args.output if args.output else 1, "w", encoding=args.out_enc) as f:
        f.write(output_str)

    in_from = args.input if args.input else "<stdin>"
    out_to = args.output if args.output else "<stdout>"
    if sys.stderr.isatty():
        if output_str and not output_str.endswith("\n"):
            print()
        print(f"Segmentation completed ({mode}, HMM:{hmm if mode != 'full' else 'None'}): {in_from} -> {out_to}",
              file=sys.stderr)
    return 0


def main():
    parser = argparse.ArgumentParser(
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
        description="High-performance Simplified ↔ Traditional Chinese conversion and segmentation CLI powered by opencc-jieba-rs"
    )
    subparsers = parser.add_subparsers(dest='command', required=True)

    # ------------------
    # Convert subcommand
    # ------------------
    parser_convert = subparsers.add_parser('convert', help='Convert text using OpenCC + Jieba',
                                           formatter_class=argparse.ArgumentDefaultsHelpFormatter)
    parser_convert.add_argument('-i', '--input', metavar='<file>',
                                help='Read original text from <file>.')
    parser_convert.add_argument('-o', '--output', metavar='<file>',
                                help='Write converted text to <file>.')
    parser_convert.add_argument('-c', '--config', metavar='<conversion>',
                                help='Conversion configuration: [s2t|s2tw|s2twp|s2hk|t2s|tw2s|tw2sp|hk2s|jp2t|t2jp]')
    parser_convert.add_argument('-p', '--punct', action='store_true', default=False,
                                help='Punctuation conversion')
    parser_convert.add_argument('--in-enc', metavar='<encoding>', default='UTF-8',
                                help='Encoding for input')
    parser_convert.add_argument('--out-enc', metavar='<encoding>', default='UTF-8',
                                help='Encoding for output')
    parser_convert.set_defaults(func=subcommand_convert)

    # -----------------
    # office subcommand
    # -----------------
    parser_office = subparsers.add_parser(
        "office",
        help="Convert Office document and EPUB Chinese text using OpenCC",
    )
    parser_office.add_argument(
        "-i",
        "--input",
        metavar="<file>",
        help="Input Office document from <file>.",
    )
    parser_office.add_argument(
        "-o",
        "--output",
        metavar="<file>",
        help="Output Office document to <file>.",
    )
    parser_office.add_argument(
        "-c",
        "--config",
        metavar="<conversion>",
        help=(
            "conversion: "
            "s2t|s2tw|s2twp|s2hk|t2s|tw2s|tw2sp|hk2s|jp2t|t2jp"
        ),
    )
    parser_office.add_argument(
        "-p",
        "--punct",
        action="store_true",
        default=False,
        help="Enable punctuation conversion. (Default: False)",
    )
    parser_office.add_argument(
        "-f",
        "--format",
        metavar="<format>",
        help="Target Office format (e.g., docx, xlsx, pptx, odt, ods, odp, epub)",
    )
    parser_office.add_argument(
        "-k",
        "--keep-font",
        action="store_true",
        default=False,
        help="Preserve font-family information in Office content",
    )
    parser_office.set_defaults(func=subcommand_office)

    # ------------------
    # Segment subcommand
    # ------------------
    parser_segment = subparsers.add_parser(
        "segment",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
        help="Segment Chinese text using Jieba"
    )
    parser_segment.add_argument("-i", "--input", metavar="<file>",
                                help="Read input text from <file>.")
    parser_segment.add_argument("-o", "--output", metavar="<file>",
                                help="Write segmented text to <file>.")
    parser_segment.add_argument("-d", "--delim", metavar="<char>", default=" ",
                                help="Delimiter to join segments")
    parser_segment.add_argument("-s", "--separator", metavar="<char>", default="/",
                                help="Separator for segment mode: tag")
    parser_segment.add_argument('--no-hmm', action='store_true', default=False,
                                help='Disable HMM')
    parser_segment.add_argument(
        "-m", "--mode",
        choices=["cut", "search", "full", "tag"],
        default="cut",
        help="Segmentation mode"
    )
    parser_segment.add_argument("--in-enc", metavar="<encoding>", default="UTF-8",
                                help="Encoding for input")
    parser_segment.add_argument("--out-enc", metavar="<encoding>", default="UTF-8",
                                help="Encoding for output")
    parser_segment.set_defaults(func=subcommand_segment)

    args = parser.parse_args()
    return args.func(args)


if __name__ == '__main__':
    sys.exit(main())
