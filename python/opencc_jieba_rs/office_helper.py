"""
OpenCC-based Office and EPUB document converter.

This module provides helper functions to convert and repackage Office documents and EPUBs,
supporting optional font preservation.

Supported formats: docx, xlsx, pptx, odt, ods, odp, epub.

Author
------
https://github.com/laisuk
"""
import os
import re
import shutil
import tempfile
import zipfile
from pathlib import Path
from typing import Tuple, List, Optional

# Global list of supported Office document formats
OFFICE_FORMATS = [
    "docx",  # Word
    "xlsx",  # Excel
    "pptx",  # PowerPoint
    "odt",  # OpenDocument Text
    "ods",  # OpenDocument Spreadsheet
    "odp",  # OpenDocument Presentation
    "epub",  # eBook (XHTML-based)
]


def convert_office_doc(
        input_path: str,
        output_path: str,
        office_format: str,
        converter,
        punctuation: bool = False,
        keep_font: bool = False,
) -> Tuple[bool, str]:
    """
    Converts an Office document by applying OpenCC conversion on specific XML parts.
    Optionally preserves original_value font names to prevent them from being altered.

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
    input_path = Path(input_path)
    output_path = Path(output_path)

    # --- NEW: normalized temp root and pre-created working dir
    temp_root = _normalized_temp_root()
    temp_dir = Path(tempfile.mkdtemp(prefix=f"{office_format}_temp_", dir=temp_root))

    try:
        with zipfile.ZipFile(input_path, 'r') as archive:
            for entry in archive.infolist():
                try:
                    dest_path = _safe_zip_join(str(temp_dir), entry.filename)
                except ValueError as ve:
                    return False, f"❌ {ve}"

                if entry.is_dir():
                    dest_path.mkdir(parents=True, exist_ok=True)
                else:
                    dest_path.parent.mkdir(parents=True, exist_ok=True)
                    with archive.open(entry) as src, open(dest_path, 'wb') as dst:
                        shutil.copyfileobj(src, dst)  # type: ignore

        target_paths = _get_target_xml_paths(office_format, temp_dir)
        if not target_paths:
            return False, f"❌ Unsupported or invalid format: {office_format}"

        converted_count = 0

        for relative_path in target_paths:
            full_path = temp_dir / relative_path
            if not full_path.is_file():
                continue

            xml_content = full_path.read_text(encoding="utf-8")

            font_map = {}
            if keep_font:
                pattern = _get_font_regex_pattern(office_format)
                font_counter = 0

                if pattern:
                    def replace_font(match):
                        nonlocal font_counter
                        font_key = f"__F_O_N_T_{font_counter}__"
                        original_value = match.group(2)
                        font_map[font_key] = original_value
                        font_counter += 1
                        return f"{match.group(1)}{font_key}{match.group(3)}"

                    xml_content = re.sub(pattern, replace_font, xml_content)

            converted = converter.convert(xml_content, punctuation=punctuation)

            if keep_font:
                for marker, original in font_map.items():
                    converted = converted.replace(marker, original)

            full_path.write_text(converted, encoding="utf-8")
            converted_count += 1

        if converted_count == 0:
            return False, f"⚠️ No valid XML fragments were found. Is the format '{office_format}' correct?"

        # Ensure output path is clear
        try:
            output_path.unlink(missing_ok=True)  # Python 3.8+: ok
        except TypeError:
            if output_path.exists():
                output_path.unlink()

        if office_format == "epub":
            return create_epub_zip_with_spec(temp_dir, output_path)
        else:
            with zipfile.ZipFile(output_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
                for file in temp_dir.rglob("*"):
                    if file.is_file():
                        # Use forward slashes inside the zip
                        archive.write(file, os.path.normpath(file.relative_to(temp_dir).as_posix()))

        return True, f"✅ Successfully converted {converted_count} fragment(s) in {office_format} document."

    except Exception as ex:
        return False, f"❌ Conversion failed: {ex}"
    finally:
        if temp_dir.exists():
            # Robust cleanup on Windows (readonly files)
            def _onerror(func, path, _):
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
    # Normalize
    base_dir_norm = os.path.normpath(base_dir)
    dest = os.path.normpath(os.path.join(base_dir_norm, member))
    # Ensure dest remains inside base_dir
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
    elif office_format == "xlsx":
        return [Path("xl/sharedStrings.xml")]
    elif office_format == "pptx":
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
    elif office_format in ("odt", "ods", "odp"):
        return [Path("content.xml")]
    elif office_format == "epub":
        return [
            path.relative_to(base_dir)
            for path in base_dir.rglob("*")
            if path.suffix.lower() in (".xhtml", ".html", ".opf", ".ncx")
        ]
    return None


def _get_font_regex_pattern(office_format: str) -> Optional[str]:
    """
    Returns a regex pattern to match font-family attributes for the given format.

    Args:
        office_format: The document format.

    Returns:
        Regex string or None if not applicable.
    """
    return {
        "docx": r'(w:(?:eastAsia|ascii|hAnsi|cs)=")([^"]+)(")',
        "xlsx": r'(val=")(.*?)(")',
        "pptx": r'(typeface=")(.*?)(")',
        "odt": r'((?:style:font-name(?:-asian|-complex)?|svg:font-family|style:name)=["\'])([^"\']+)(["\'])',
        "ods": r'((?:style:font-name(?:-asian|-complex)?|svg:font-family|style:name)=["\'])([^"\']+)(["\'])',
        "odp": r'((?:style:font-name(?:-asian|-complex)?|svg:font-family|style:name)=["\'])([^"\']+)(["\'])',
        "epub": r'(font-family\s*:\s*)([^;]+)([;])?',
    }.get(office_format)


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
            # 1) Write mimetype first (must be uncompressed per EPUB spec)
            epub.write(mime_path, "mimetype", compress_type=zipfile.ZIP_STORED)

            # 2) Write remaining files deterministically
            for file in sorted(source_dir.rglob("*")):
                if not file.is_file():
                    continue

                arc_name = file.relative_to(source_dir).as_posix()

                # Skip mimetype (already written)
                if arc_name == "mimetype":
                    continue

                epub.write(file, arc_name, compress_type=zipfile.ZIP_DEFLATED)

        return True, "✅ EPUB archive created successfully."
    except Exception as ex:
        return False, f"❌ Failed to create EPUB: {ex}"
