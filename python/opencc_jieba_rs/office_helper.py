"""
OpenCC-based Office and EPUB document converter.

This module provides services functions to convert and repackage Office documents and EPUBs,
supporting optional font preservation.

Supported formats: docx, xlsx, pptx, odt, ods, odp, epub.

Author
------
https://github.com/laisuk
"""
from __future__ import annotations

import os
import re
import shutil
import tempfile
import zipfile
from pathlib import Path
from typing import Dict, IO, List, Match, Optional, Protocol, Tuple

# Global list of supported Office document formats
OFFICE_FORMATS: List[str] = [
    "docx",  # Word
    "xlsx",  # Excel
    "pptx",  # PowerPoint
    "odt",  # OpenDocument Text
    "ods",  # OpenDocument Spreadsheet
    "odp",  # OpenDocument Presentation
    "epub",  # eBook (XHTML-based)
]


class OpenCCLike(Protocol):
    def convert(self, text: str, punctuation: bool = False) -> str:
        ...


_XLSX_INLINE_STRING_CELL_RE: re.Pattern[str] = re.compile(
    r"<c\b(?=[^>]*\bt=(?:\"inlineStr\"|'inlineStr'))[^>]*>.*?</c>",
    re.DOTALL,
)

_XLSX_TEXT_NODE_RE: re.Pattern[str] = re.compile(
    r"(<t\b[^>]*>)(.*?)(</t>)",
    re.DOTALL,
)


def convert_office_doc(
        input_path: str,
        output_path: str,
        office_format: str,
        converter: OpenCCLike,
        punctuation: bool = False,
        keep_font: bool = False,
) -> Tuple[bool, str]:
    """
    Converts an Office document by applying OpenCC conversion on specific XML parts.
    Optionally preserves original font names to prevent them from being altered.

    Args:
        input_path: Path to input .docx, .xlsx, .pptx, .odt, .epub, etc.
        output_path: Path for the output converted document.
        office_format: One of 'docx', 'xlsx', 'pptx', 'odt', 'ods', 'odp', 'epub'.
        converter: An object with a method `convert(text, punctuation=True|False)`.
        punctuation: Whether to convert punctuation.
        keep_font: If True, font names are preserved during conversion.

    Returns:
        (success: bool, message: str)
    """
    input_path = str(Path(input_path))
    output_path = str(Path(output_path))
    office_format = office_format.lower()

    temp_root = _normalized_temp_root()
    temp_dir = Path(tempfile.mkdtemp(prefix=f"{office_format}_temp_", dir=temp_root))

    try:
        with zipfile.ZipFile(input_path, "r") as archive:
            for entry in archive.infolist():
                try:
                    dest_path = _safe_zip_join(str(temp_dir), entry.filename)
                except ValueError as ve:
                    return False, f"❌ {ve}"

                if entry.is_dir():
                    dest_path.mkdir(parents=True, exist_ok=True)
                else:
                    parent = dest_path.parent
                    parent.mkdir(parents=True, exist_ok=True)
                    with archive.open(entry) as src_raw, open(dest_path, "wb") as dst_raw:
                        src: IO[bytes] = src_raw
                        dst: IO[bytes] = dst_raw
                        shutil.copyfileobj(src, dst)

        target_paths = _get_target_xml_paths(office_format, temp_dir)
        if not target_paths:
            return False, f"❌ Unsupported or invalid format: {office_format}"

        converted_count = 0

        for relative_path in target_paths:
            full_path = temp_dir / relative_path
            if not full_path.is_file():
                continue

            xml_content = full_path.read_text(encoding="utf-8")

            font_map: Dict[str, str] = {}
            if keep_font and _should_mask_fonts(office_format, relative_path):
                pattern = _get_font_regex_pattern(office_format)
                font_counter = 0

                if pattern is not None:
                    def replace_font(match: Match[str]) -> str:
                        nonlocal font_counter
                        font_key = f"__F_O_N_T_{font_counter}__"
                        original_value = match.group(2)
                        font_map[font_key] = original_value
                        font_counter += 1

                        group3 = match.group(3)
                        suffix = group3 if group3 is not None else ""

                        return f"{match.group(1)}{font_key}{suffix}"

                    xml_content = pattern.sub(replace_font, xml_content)

            converted: str
            if office_format == "xlsx":
                converted = _convert_xlsx_xml_part(
                    xml_content,
                    relative_path,
                    converter,
                    punctuation,
                )
            else:
                converted = converter.convert(xml_content, punctuation=punctuation)

            if keep_font and font_map:
                for marker, original in font_map.items():
                    converted = converted.replace(marker, original)

            full_path.write_text(converted, encoding="utf-8")
            converted_count += 1

        if converted_count == 0:
            return False, f"⚠️ No valid XML fragments were found. Is the format '{office_format}' correct?"

        try:
            Path(output_path).unlink(missing_ok=True)
        except TypeError:
            if Path(output_path).exists():
                Path(output_path).unlink()

        if office_format == "epub":
            return create_epub_zip_with_spec(temp_dir, Path(output_path))

        with zipfile.ZipFile(output_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for file in temp_dir.rglob("*"):
                if file.is_file():
                    archive.write(file, os.path.normpath(file.relative_to(temp_dir).as_posix()))

        return True, f"✅ Successfully converted {converted_count} fragment(s) in {office_format} document."

    except Exception as ex:
        return False, f"❌ Conversion failed: {ex}"
    finally:
        if temp_dir.exists():
            # Robust cleanup on Windows (readonly files)
            def _onerror(func, path, _exc):
                try:
                    os.chmod(path, 0o700)
                    func(path)
                except (PermissionError, OSError):
                    pass

            shutil.rmtree(temp_dir, onerror=_onerror)


def _normalized_temp_root() -> str:
    # Normalize temp root path string to avoid Windows resolve() issues (e.g., R:\Temp)
    return os.path.normpath(os.path.abspath(tempfile.gettempdir()))


def _safe_zip_join(base_dir: str, member: str) -> Path:
    """
    Safely join a zip member path under base_dir without using Path.resolve(),
    preventing Zip Slip via commonpath check.
    """
    base_dir_norm = os.path.normpath(base_dir)
    dest = os.path.normpath(os.path.join(base_dir_norm, member))
    if os.path.commonpath([base_dir_norm, dest]) != base_dir_norm:
        raise ValueError(f"Unsafe ZIP path detected: {member}")
    return Path(dest)


def _get_target_xml_paths(office_format: str, base_dir: Path) -> Optional[List[Path]]:
    """
    Returns a list of XML file paths within the extracted Office/EPUB directory
    that should be converted for the given format.

    Args:
        office_format: The document format (e.g., 'docx', 'xlsx', 'epub').
        base_dir: The root directory of the extracted archive.

    Returns:
        List of relative XML file paths to process, or None if unsupported.
    """
    if office_format == "docx":
        return [Path("word/document.xml")]

    if office_format == "xlsx":
        targets: List[Path] = []

        shared_strings = base_dir / "xl" / "sharedStrings.xml"
        if shared_strings.is_file():
            targets.append(Path("xl/sharedStrings.xml"))

        worksheets_dir = base_dir / "xl" / "worksheets"
        if worksheets_dir.is_dir():
            targets.extend(
                path.relative_to(base_dir)
                for path in worksheets_dir.rglob("*.xml")
                if path.is_file()
            )

        return targets

    if office_format == "pptx":
        ppt_dir = base_dir / "ppt"
        if ppt_dir.is_dir():
            return [
                path.relative_to(base_dir)
                for path in ppt_dir.rglob("*.xml")
                if path.name.startswith("slide")
                   or "notesSlide" in path.name
                   or "slideMaster" in path.name
                   or "slideLayout" in path.name
                   or "comment" in path.name
            ]

    if office_format in ("odt", "ods", "odp"):
        return [Path("content.xml")]

    if office_format == "epub":
        return [
            path.relative_to(base_dir)
            for path in base_dir.rglob("*")
            if path.suffix.lower() in (".xhtml", ".html", ".opf", ".ncx")
        ]

    return None


def _should_mask_fonts(office_format: str, relative_path: Path) -> bool:
    """
    Returns whether font masking should be applied for the given part.

    For XLSX, masking is limited to sharedStrings.xml only.
    """
    if office_format != "xlsx":
        return True

    normalized = relative_path.as_posix()
    return normalized.lower() == "xl/sharedStrings.xml"


def _is_xlsx_worksheet_path(relative_path: Path) -> bool:
    normalized = relative_path.as_posix()
    return normalized.startswith("xl/worksheets/") and normalized.endswith(".xml")


def _convert_xlsx_xml_part(
        xml_content: str,
        relative_path: Path,
        converter: OpenCCLike,
        punctuation: bool,
) -> str:
    """
    Converts an XLSX XML part using narrow rules:
    - sharedStrings.xml -> whole-file conversion
    - worksheet XML -> only inline-string cell text nodes
    - other XLSX XML parts -> unchanged
    """
    normalized = relative_path.as_posix()

    if normalized.lower() == "xl/sharedStrings.xml":
        return converter.convert(xml_content, punctuation=punctuation)

    if _is_xlsx_worksheet_path(relative_path):
        def replace_cell(cell_match: Match[str]) -> str:
            cell_xml = cell_match.group(0)

            def replace_text(text_match: Match[str]) -> str:
                open_tag = text_match.group(1)
                inner_text = text_match.group(2)
                close_tag = text_match.group(3)

                if not inner_text:
                    return text_match.group(0)

                converted_text = converter.convert(inner_text, punctuation=punctuation)
                return f"{open_tag}{converted_text}{close_tag}"

            return _XLSX_TEXT_NODE_RE.sub(replace_text, cell_xml)

        return _XLSX_INLINE_STRING_CELL_RE.sub(replace_cell, xml_content)

    return xml_content


def _get_font_regex_pattern(office_format: str) -> Optional[re.Pattern[str]]:
    """
    Returns a regex pattern to match font-family attributes for the given format.

    Args:
        office_format: The document format.

    Returns:
        Compiled regex pattern or None if not applicable.
    """
    pattern_map: Dict[str, str] = {
        "docx": r'(w:(?:eastAsia|ascii|hAnsi|cs)=")([^"]+)(")',
        "xlsx": r'(val=")(.*?)(")',
        "pptx": r'(typeface=")(.*?)(")',
        "odt": r'((?:style:font-name(?:-asian|-complex)?|svg:font-family|style:name)=["\'])([^"\']+)(["\'])',
        "ods": r'((?:style:font-name(?:-asian|-complex)?|svg:font-family|style:name)=["\'])([^"\']+)(["\'])',
        "odp": r'((?:style:font-name(?:-asian|-complex)?|svg:font-family|style:name)=["\'])([^"\']+)(["\'])',
        "epub": r'(font-family\s*:\s*)([^;"\']+)([;"\'])?',
    }
    pattern = pattern_map.get(office_format)
    return re.compile(pattern) if pattern is not None else None


def create_epub_zip_with_spec(source_dir: Path, output_path: Path) -> Tuple[bool, str]:
    """
    Creates a valid EPUB-compliant ZIP archive.
    Ensures `mimetype` is the first file and uncompressed.

    Args:
        source_dir: The unpacked EPUB directory.
        output_path: Final path to .epub file.

    Returns:
        Tuple of (success, message)
    """
    mime_path = source_dir / "mimetype"

    try:
        if not mime_path.is_file():
            return False, "❌ 'mimetype' file is missing. EPUB requires it as the first entry."

        with zipfile.ZipFile(output_path, "w") as epub:
            epub.write(mime_path, "mimetype", compress_type=zipfile.ZIP_STORED)

            for file in sorted(source_dir.rglob("*")):
                if not file.is_file():
                    continue

                arc_name = file.relative_to(source_dir).as_posix()
                if arc_name == "mimetype":
                    continue

                epub.write(file, arc_name, compress_type=zipfile.ZIP_DEFLATED)

        return True, "✅ EPUB archive created successfully."
    except Exception as ex:
        return False, f"❌ Failed to create EPUB: {ex}"
