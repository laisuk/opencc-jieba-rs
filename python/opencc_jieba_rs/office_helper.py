"""
Office and EPUB package conversion helpers.

This module provides an engine-independent conversion pipeline for ZIP-based
Office formats and EPUB. Callers supply an ``OfficeTextConverter`` callable that
owns text transformation policy, while this module owns package extraction and
rebuild, target-part selection, XLSX inline-string handling, optional font
preservation, EPUB packaging rules, ZIP-path safety, output validation, and
transactional publication.

Supported formats
-----------------
``docx``, ``xlsx``, ``pptx``, ``odt``, ``ods``, ``odp``, and ``epub``.

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
from typing import Callable, Dict, IO, List, Match, Optional, Tuple

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

_XLSX_INLINE_STRING_CELL_RE: re.Pattern[str] = re.compile(
    r"<c\b(?=[^>]*\bt=(?:\"inlineStr\"|'inlineStr'))[^>]*>.*?</c>",
    re.DOTALL,
)

_XLSX_TEXT_NODE_RE: re.Pattern[str] = re.compile(
    r"(<t\b[^>]*>)(.*?)(</t>)",
    re.DOTALL,
)

OfficeTextConverter = Callable[[str], str]


def convert_office_doc(
        input_path: str,
        output_path: Optional[str],
        office_format: str,
        office_text_converter: OfficeTextConverter,
        keep_font: bool = False,
) -> Tuple[bool, str]:
    """
    Convert an Office or EPUB package with a caller-supplied text transformer.

    The document layer is independent of any specific OpenCC implementation.
    ``office_text_converter`` is invoked only for selected text-bearing package
    content. Package structure, archive handling, format-specific part selection,
    XLSX inline-string rules, optional font preservation, EPUB conformance, and
    output publication remain owned by this module.

    Existing output is not replaced until a complete candidate archive has been
    created and validated successfully.

    Parameters
    ----------
    input_path:
        Path to the source ``.docx``, ``.xlsx``, ``.pptx``, ``.odt``, ``.ods``,
        ``.odp``, or ``.epub`` package.
    output_path:
        Destination path. If ``None``, a sibling file named
        ``<input-stem>_converted<original-extension>`` is used.
    office_format:
        Logical package format. Matching is case-insensitive.
    office_text_converter:
        Callable receiving selected text and returning its transformed replacement.
        The callable must return a string.
    keep_font:
        Preserve recognized font-family attributes with temporary markers while
        conversion is performed.

    Returns
    -------
    Tuple[bool, str]
        ``(True, message)`` on success, otherwise ``(False, error_message)``.
    """
    input_path = str(Path(input_path))
    output_path = _normalize_output_path(input_path, output_path, office_format)
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
                    office_text_converter,
                )
            else:
                converted = _apply_text_converter(office_text_converter, xml_content)

            if keep_font and font_map:
                converted = _restore_fonts(converted, font_map)

            full_path.write_text(converted, encoding="utf-8")
            converted_count += 1

        if converted_count == 0:
            return False, f"⚠️ No valid XML fragments were found. Is the format '{office_format}' correct?"

        final_output = Path(output_path)
        temp_output = _sibling_temp_output_path(final_output)

        try:
            if office_format == "epub":
                success, message = create_epub_zip_with_spec(temp_dir, temp_output)
                if not success:
                    return False, message
            else:
                _create_zip_from_directory(temp_dir, temp_output)

            _validate_zip_file(temp_output)
            os.replace(str(temp_output), str(final_output))
        finally:
            try:
                if temp_output.exists():
                    temp_output.unlink()
            except OSError:
                pass

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


def _restore_fonts(converted: str, font_map: Dict[str, str]) -> str:
    """Restore markers in one pass without rescanning inserted font values."""
    if not font_map:
        return converted

    prefix = "__F_O_N_T_"
    chunks: List[str] = []
    cursor = 0
    while True:
        start = converted.find(prefix, cursor)
        if start < 0:
            break
        chunks.append(converted[cursor:start])
        prefix_end = start + len(prefix)
        end = converted.find("__", prefix_end)
        font = font_map.get(converted[start:end + 2]) if end >= 0 else None
        if font is not None:
            chunks.append(font)
            cursor = end + 2
        else:
            # Preserve unknown/incomplete markers and still find later known ones.
            chunks.append(prefix)
            cursor = prefix_end
    chunks.append(converted[cursor:])
    return "".join(chunks)


def _normalized_temp_root() -> str:
    # Normalize temp root path string to avoid Windows resolve() issues (e.g., R:\Temp)
    return os.path.normpath(os.path.abspath(tempfile.gettempdir()))


def _normalize_output_path(
        input_path: str,
        output_path: Optional[str],
        office_format: str,
) -> str:
    if output_path is not None:
        return str(Path(output_path))

    input_file = Path(input_path)
    input_ext = input_file.suffix or f".{office_format.lower()}"
    return str(input_file.with_name(f"{input_file.stem}_converted{input_ext}"))


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
            targets: List[Path] = []

            for path in ppt_dir.rglob("*.xml"):
                if not path.is_file():
                    continue

                relative_path = path.relative_to(base_dir)
                if _is_pptx_target_xml_path(relative_path):
                    targets.append(relative_path)

            return targets

    if office_format in ("odt", "ods", "odp"):
        return [Path("content.xml")]

    if office_format == "epub":
        return [
            path.relative_to(base_dir)
            for path in base_dir.rglob("*")
            if path.suffix.lower() in (".xhtml", ".html", ".opf", ".ncx")
        ]

    return None


def _is_pptx_target_xml_path(relative_path: Path) -> bool:
    """
    Return whether a PPTX XML part is intended for text conversion.

    Matching uses normalized package-relative paths rather than broad filename
    substring tests so unrelated XML parts are left untouched.
    """
    normalized = relative_path.as_posix().lower()

    if not normalized.endswith(".xml"):
        return False

    return (
            normalized.startswith("ppt/slides/")
            or normalized.startswith("ppt/notesslides/")
            or normalized.startswith("ppt/slidemasters/")
            or normalized.startswith("ppt/slidelayouts/")
            or normalized.startswith("ppt/comments/")
            or normalized == "ppt/commentauthors.xml"
    )


def _sibling_temp_output_path(output_path: Path) -> Path:
    """
    Create a unique candidate-output path beside the final destination.

    Keeping the temporary archive in the same directory allows ``os.replace``
    to publish it without crossing filesystems.
    """
    final_path = Path(os.path.abspath(str(output_path)))
    final_path.parent.mkdir(parents=True, exist_ok=True)

    fd, temp_name = tempfile.mkstemp(
        prefix=f".{final_path.name}.",
        suffix=".tmp",
        dir=str(final_path.parent),
    )
    os.close(fd)

    temp_path = Path(temp_name)
    try:
        temp_path.unlink()
    except OSError:
        pass

    return temp_path


def _create_zip_from_directory(source_dir: Path, output_path: Path) -> None:
    """Create a normal deflated Office/ODF ZIP package from ``source_dir``."""
    with zipfile.ZipFile(output_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for file in source_dir.rglob("*"):
            if not file.is_file():
                continue

            archive.write(file, file.relative_to(source_dir).as_posix())


def _validate_zip_file(path: Path) -> None:
    """
    Validate that ``path`` is a readable ZIP archive with no CRC failures.

    Raises ``zipfile.BadZipFile`` when the candidate archive is unreadable or a
    member fails its CRC check.
    """
    with zipfile.ZipFile(path, "r") as archive:
        bad_member = archive.testzip()
        if bad_member is not None:
            raise zipfile.BadZipFile(
                f"CRC check failed for ZIP entry: {bad_member}"
            )


def _should_mask_fonts(office_format: str, relative_path: Path) -> bool:
    """
    Returns whether font masking should be applied for the given part.

    For XLSX, masking is limited to sharedStrings.xml only.
    """
    if office_format != "xlsx":
        return True

    normalized = relative_path.as_posix()
    return normalized.lower() == "xl/sharedstrings.xml"


def _is_xlsx_worksheet_path(relative_path: Path) -> bool:
    normalized = relative_path.as_posix()
    return normalized.startswith("xl/worksheets/") and normalized.endswith(".xml")


def _convert_xlsx_xml_part(
        xml_content: str,
        relative_path: Path,
        office_text_converter: OfficeTextConverter,
) -> str:
    """
    Converts an XLSX XML part using narrow rules:
    - sharedStrings.xml -> whole-file conversion
    - worksheet XML -> only inline-string cell text nodes
    - other XLSX XML parts -> unchanged
    """
    normalized = relative_path.as_posix()

    if normalized.lower() == "xl/sharedstrings.xml":
        return _apply_text_converter(office_text_converter, xml_content)

    if _is_xlsx_worksheet_path(relative_path):
        def replace_cell(cell_match: Match[str]) -> str:
            cell_xml = cell_match.group(0)

            def replace_text(text_match: Match[str]) -> str:
                open_tag = text_match.group(1)
                inner_text = text_match.group(2)
                close_tag = text_match.group(3)

                if not inner_text:
                    return text_match.group(0)

                converted_text = _apply_text_converter(office_text_converter, inner_text)
                return f"{open_tag}{converted_text}{close_tag}"

            return _XLSX_TEXT_NODE_RE.sub(replace_text, cell_xml)

        return _XLSX_INLINE_STRING_CELL_RE.sub(replace_cell, xml_content)

    return xml_content


def _apply_text_converter(
        office_text_converter: OfficeTextConverter,
        text: str,
) -> str:
    """Apply the text converter and reject an invalid ``None`` result."""
    converted = office_text_converter(text)
    if converted is None:
        raise ValueError("Office text converter returned None.")
    return converted


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
    Create an EPUB-compliant ZIP package.

    The EPUB ``mimetype`` entry is written first and stored without compression.
    All remaining package files are written with deflate compression.

    This function creates the archive only. Callers that require transactional
    publication should write to a temporary path, validate the archive, and then
    replace the final destination.

    Parameters
    ----------
    source_dir:
        Root directory containing the unpacked EPUB package.
    output_path:
        Path of the ZIP/EPUB archive to create.

    Returns
    -------
    Tuple[bool, str]
        ``(True, message)`` on success, otherwise ``(False, error_message)``.
    """
    mime_path = source_dir / "mimetype"

    try:
        if not mime_path.is_file():
            return False, "❌ 'mimetype' file is missing. EPUB requires it as the first entry."

        output_path.parent.mkdir(parents=True, exist_ok=True)

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
