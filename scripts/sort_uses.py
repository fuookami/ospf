#!/usr/bin/env python3
"""
Rust use statement sorter for ospf-rust project.

Sorts the top-level `use` block in .rs files according to project convention:
  std → third-party → ospf_rust_base → ospf_rust_multiarray → ospf_rust_math
  → ospf_rust_quantities → ospf_rust_core → ospf_rust_framework
  → crate:: / super:: (internal)

Only sorts the contiguous top-level use block at the top of the file.
Does NOT touch use statements inside fn bodies, impl blocks, etc.

Usage:
  python sort_uses.py <file.rs> [--dry-run]
  python sort_uses.py <directory> [--dry-run]
"""

import re
import sys
import os
import argparse
from typing import List, Tuple, Optional


def use_priority(path: str) -> Tuple[int, str]:
    """Return (priority, path) for sorting use statements."""
    if path.startswith("std::"):
        return (0, path)
    elif path.startswith("ospf_rust_base"):
        return (2, path)
    elif path.startswith("ospf_rust_multiarray"):
        return (3, path)
    elif path.startswith("ospf_rust_math"):
        return (4, path)
    elif path.startswith("ospf_rust_quantities"):
        return (5, path)
    elif path.startswith("ospf_rust_core"):
        return (6, path)
    elif path.startswith("ospf_rust_framework"):
        return (7, path)
    elif path.startswith("crate::") or path.startswith("super::") or path.startswith("self::"):
        return (8, path)
    else:
        # Third-party crate
        return (1, path)


def extract_use_path(line: str) -> Optional[str]:
    """Extract the path from a use statement line."""
    m = re.match(r'\s*(?:#\[.*?\]\s*)?use\s+(.+?);', line)
    if m:
        return m.group(1).strip()
    return None


def is_use_line(line: str) -> bool:
    """Check if a line is a use statement."""
    stripped = line.strip()
    return bool(re.match(r'use\s+', stripped))


def is_cfg_attr_line(line: str) -> bool:
    """Check if a line is a #[cfg(...)] attribute."""
    stripped = line.strip()
    return bool(re.match(r'#\[cfg\(', stripped))


def find_top_use_block(lines: List[str]) -> Tuple[int, int]:
    """
    Find the top-level use block boundaries.

    Returns (start_idx, end_idx) where:
    - start_idx is the first line that is a use statement (or its #[cfg] attr)
    - end_idx is the last line that is a use statement

    The use block is the contiguous region starting from the first use line
    (skipping blank lines between use lines) up to the last use line before
    a non-use, non-blank line.
    """
    # Find the first use line
    first_use = None
    for i, line in enumerate(lines):
        stripped = line.strip()
        # Skip crate-level attributes, extern crate, comments, blank lines
        if stripped == '':
            continue
        if stripped.startswith('#!['):
            continue
        if stripped.startswith('#['):
            # Could be #[macro_use] or #[cfg] before use
            if is_cfg_attr_line(line):
                # Check if next non-blank line is a use
                j = i + 1
                while j < len(lines) and lines[j].strip() == '':
                    j += 1
                if j < len(lines) and is_use_line(lines[j]):
                    first_use = i
                    break
            elif stripped.startswith('#[macro_use]') or stripped.startswith('#[allow('):
                continue
            else:
                continue
        if stripped.startswith('extern crate'):
            continue
        if stripped.startswith('//'):
            continue
        if is_use_line(line):
            first_use = i
            break
        # If we hit a non-use, non-attribute line, the use block is done
        break

    if first_use is None:
        return (-1, -1)

    # Find the end of the use block:
    # Continue from first_use, collecting use lines and blank lines between them.
    # Stop when we hit a non-use, non-blank, non-cfg-attr line.
    # Also check for #[cfg] attributes before use lines.
    last_use = first_use
    i = first_use
    while i < len(lines):
        stripped = lines[i].strip()

        if stripped == '':
            # Blank line - check if there's another use after it
            j = i + 1
            # Skip consecutive blank lines
            while j < len(lines) and lines[j].strip() == '':
                j += 1
            if j < len(lines) and (is_use_line(lines[j]) or is_cfg_attr_line(lines[j])):
                i = j
                continue
            else:
                # No more use lines after blank line(s)
                break

        if is_cfg_attr_line(lines[i]):
            # Check if next non-blank line is a use
            j = i + 1
            while j < len(lines) and lines[j].strip() == '':
                j += 1
            if j < len(lines) and is_use_line(lines[j]):
                i = j
                if is_use_line(lines[i]):
                    last_use = i
                i += 1
                continue
            else:
                break

        if is_use_line(lines[i]):
            last_use = i
            i += 1
            continue

        # Non-use, non-blank, non-cfg line -> end of use block
        break

    return (first_use, last_use)


def sort_uses_in_file(filepath: str, dry_run: bool = False) -> bool:
    """Sort use statements in a single .rs file. Returns True if changes were made."""
    with open(filepath, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    start_idx, end_idx = find_top_use_block(lines)
    if start_idx < 0 or end_idx < 0:
        return False

    # Collect use entries from the block
    # Each entry is (list_of_raw_lines, use_path_string_or_None)
    entries = []
    i = start_idx

    while i <= end_idx:
        line = lines[i]
        stripped = line.strip()

        if stripped == '':
            i += 1
            continue

        if is_cfg_attr_line(line):
            attr_line = line
            j = i + 1
            while j <= end_idx and lines[j].strip() == '':
                j += 1
            if j <= end_idx and is_use_line(lines[j]):
                use_path = extract_use_path(lines[j])
                entries.append(([line, lines[j]], use_path))
                i = j + 1
                continue
            else:
                # Orphan cfg attr, skip
                i += 1
                continue

        if is_use_line(line):
            use_path = extract_use_path(line)
            entries.append(([line], use_path))
            i += 1
            continue

        # Unexpected line in use block
        i += 1

    if not entries:
        return False

    # Sort entries by priority
    def sort_key(entry):
        _, path = entry
        if path is None:
            return (99, "")
        return use_priority(path)

    sorted_entries = sorted(entries, key=sort_key)

    # Build new use lines: continuous, no blank lines between
    new_use_lines = []
    for lines_list, _ in sorted_entries:
        new_use_lines.extend(lines_list)

    # Check if anything changed
    old_use_lines = []
    for i in range(start_idx, end_idx + 1):
        if lines[i].strip() != '':
            old_use_lines.append(lines[i])

    # Compare sorted vs original (ignoring blank lines in original)
    if new_use_lines == old_use_lines and len(new_use_lines) == (end_idx - start_idx + 1):
        # Same content and no blank lines to remove
        return False

    # Replace the old use block with the new one
    # We need to also remove the blank line(s) after the old block if they exist
    # and add exactly one blank line after the new use block

    # Find where to insert: replace lines[start_idx..=end_idx] with new_use_lines
    new_lines = lines[:start_idx] + new_use_lines + lines[end_idx + 1:]

    # Ensure there's exactly one blank line after the use block
    after_idx = start_idx + len(new_use_lines)
    if after_idx < len(new_lines) and new_lines[after_idx].strip() != '':
        # No blank line after use block - insert one
        new_lines = new_lines[:after_idx] + ['\n'] + new_lines[after_idx:]

    if new_lines == lines:
        return False

    if dry_run:
        print(f"  Would modify: {filepath}")
        return True

    with open(filepath, 'w', encoding='utf-8') as f:
        f.writelines(new_lines)
    return True


def process_directory(dirpath: str, dry_run: bool = False) -> Tuple[int, int]:
    """Process all .rs files in a directory recursively."""
    modified = 0
    total = 0
    for root, dirs, files in os.walk(dirpath):
        dirs[:] = [d for d in dirs if d != 'target']
        for fname in sorted(files):
            if fname.endswith('.rs'):
                fpath = os.path.join(root, fname)
                total += 1
                try:
                    if sort_uses_in_file(fpath, dry_run):
                        modified += 1
                        if dry_run:
                            pass  # already printed
                        else:
                            print(f"  Modified: {os.path.relpath(fpath, dirpath)}")
                except Exception as e:
                    print(f"  Error processing {fpath}: {e}")
    return modified, total


def main():
    parser = argparse.ArgumentParser(description='Sort Rust use statements per ospf-rust convention')
    parser.add_argument('path', help='File or directory to process')
    parser.add_argument('--dry-run', action='store_true', help='Show what would change')
    args = parser.parse_args()

    path = args.path
    if os.path.isfile(path):
        changed = sort_uses_in_file(path, args.dry_run)
        if changed:
            print(f"Would modify: {path}" if args.dry_run else f"Modified: {path}")
        else:
            print(f"No changes: {path}")
    elif os.path.isdir(path):
        modified, total = process_directory(path, args.dry_run)
        action = "Would modify" if args.dry_run else "Modified"
        print(f"\n{action}: {modified}/{total} files")
    else:
        print(f"Path not found: {path}")
        sys.exit(1)


if __name__ == '__main__':
    main()
