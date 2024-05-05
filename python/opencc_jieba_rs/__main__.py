from __future__ import print_function

import argparse
import sys
import io
from opencc_jieba_rs import OpenCC


def main():
    parser = argparse.ArgumentParser(formatter_class=argparse.ArgumentDefaultsHelpFormatter)
    parser.add_argument('-i', '--input', metavar='<file>',
                        help='Read original text from <file>.')
    parser.add_argument('-o', '--output', metavar='<file>',
                        help='Write converted text to <file>.')
    parser.add_argument('-c', '--config', metavar='<conversion>',
                        help='Conversion configuration: [s2t|s2tw|s2twp|s2hk|t2s|tw2s|tw2sp|hk2s|jp2t|t2jp]')
    parser.add_argument('-p', '--punct', action='store_true', default=False,
                        help='Punctuation conversion: True/False')
    parser.add_argument('--in-enc', metavar='<encoding>', default='UTF-8',
                        help='Encoding for input')
    parser.add_argument('--out-enc', metavar='<encoding>', default='UTF-8',
                        help='Encoding for output')
    args = parser.parse_args()

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
    print(f"Conversion completed ({args.config}): {in_from} -> {out_to}", file=sys.stderr)

    return 0


if __name__ == '__main__':
    sys.exit(main())
