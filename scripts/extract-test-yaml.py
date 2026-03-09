#!/usr/bin/env python3
"""Extract YAML inputs from YAML Test Suite files with marker conversion.

Usage:
    ./scripts/extract-test-yaml.py vendor/yaml-test-suite/DK95.yaml
    ./scripts/extract-test-yaml.py vendor/yaml-test-suite/DK95.yaml --case 3
    ./scripts/extract-test-yaml.py vendor/yaml-test-suite/DK95.yaml --case 3 --pipe-to-c

Mirrors the marker conversion and extraction logic in
crates/yamalgam-compare/src/c_baseline.rs.
"""
import argparse
import subprocess
import sys


def replace_markers(text: str) -> str:
    """Convert YAML Test Suite visual markers to actual characters."""
    text = text.replace('\u2014', '')    # em-dash (U+2014) → removed
    text = text.replace('\u2423', ' ')   # open-box (U+2423) → space
    text = text.replace('\u00BB', '\t')  # guillemet (U+00BB) → tab
    text = text.replace('\u21B5', '')    # return arrow (U+21B5) → removed
    text = text.replace('\u220E', '')    # end-of-proof (U+220E) → removed
    if text.endswith('\n'):
        # ∎ marker means no trailing newline; already stripped above
        pass
    return text


def yaml_key_value(line: str) -> str | None:
    """Extract value after yaml: key, matching the Rust logic."""
    trimmed = line.lstrip()
    if trimmed.startswith('yaml:'):
        return trimmed[5:]
    if trimmed.startswith('- '):
        after_dash = trimmed[2:].lstrip()
        if after_dash.startswith('yaml:'):
            return after_dash[5:]
    return None


def extract_yaml_from_lines(lines: list[str]) -> str | None:
    """Extract the yaml: block from a chunk of lines (one array element)."""
    in_yaml_block = False
    indent = None
    yaml_lines = []

    for line in lines:
        if in_yaml_block:
            if indent is not None:
                stripped = line.lstrip()
                current_indent = len(line) - len(stripped)
                if stripped and current_indent < indent:
                    break
                if len(line) >= indent:
                    yaml_lines.append(line[indent:])
                elif not stripped:
                    yaml_lines.append('')
                else:
                    break
            elif line.strip():
                stripped = line.lstrip()
                current_indent = len(line) - len(stripped)
                indent = current_indent
                yaml_lines.append(line[current_indent:])
        else:
            after_key = yaml_key_value(line)
            if after_key is not None:
                after_key = after_key.strip()
                if after_key in ('', '|', '|2', '|-'):
                    in_yaml_block = True
                else:
                    return replace_markers(after_key)

    if not yaml_lines:
        return None
    return replace_markers('\n'.join(yaml_lines))


def extract_all(content: str) -> list[tuple[int, str, bool]]:
    """Extract all yaml inputs from a test suite file.

    Returns list of (case_index, yaml_input, is_fail).
    """
    lines = content.splitlines()
    elements: list[list[str]] = []
    current: list[str] = []

    for line in lines:
        if not elements and not current and line.strip() == '---':
            continue
        if line.startswith('- ') and current:
            elements.append(current)
            current = []
        current.append(line)
    if current:
        elements.append(current)

    results = []
    for idx, elem_lines in enumerate(elements):
        yaml_input = extract_yaml_from_lines(elem_lines)
        # Check for fail: true
        is_fail = any('fail: true' in l for l in elem_lines
                       if not l.strip().startswith('#'))
        if yaml_input is not None:
            results.append((idx, yaml_input, is_fail))
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('file', help='YAML Test Suite file')
    parser.add_argument('--case', '-c', type=int, default=None,
                        help='Show only this case index (0-based)')
    parser.add_argument('--pipe-to-c', action='store_true',
                        help='Pipe the extracted YAML to the C harness')
    parser.add_argument('--raw', action='store_true',
                        help='Output raw bytes (no repr)')
    args = parser.parse_args()

    with open(args.file) as f:
        content = f.read()

    cases = extract_all(content)

    for idx, yaml_input, is_fail in cases:
        if args.case is not None and idx != args.case:
            continue

        fail_marker = ' [FAIL]' if is_fail else ''
        print(f'--- Case #{idx}{fail_marker} ---', file=sys.stderr)

        if args.raw:
            sys.stdout.buffer.write(yaml_input.encode())
            sys.stdout.buffer.write(b'\n')
        else:
            print(f'  repr: {repr(yaml_input)}', file=sys.stderr)
            print(f'  len:  {len(yaml_input)}', file=sys.stderr)

        if args.pipe_to_c:
            result = subprocess.run(
                ['./tools/fyaml-tokenize/fyaml-tokenize'],
                input=yaml_input.encode(),
                capture_output=True,
            )
            print(f'  C stdout:', file=sys.stderr)
            for line in result.stdout.decode(errors='replace').splitlines():
                print(f'    {line}', file=sys.stderr)
            if result.stderr:
                print(f'  C stderr:', file=sys.stderr)
                for line in result.stderr.decode(errors='replace').splitlines():
                    print(f'    {line}', file=sys.stderr)


if __name__ == '__main__':
    main()
