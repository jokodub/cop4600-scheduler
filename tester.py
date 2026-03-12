import subprocess
import sys
from pathlib import Path
import difflib
import re


def normalize(text: str) -> str:
    """
    Normalize whitespace so formatting differences don't break tests.
    """
    lines = text.splitlines()
    cleaned = []

    for line in lines:
        line = re.sub(r"\s+", " ", line.strip())
        cleaned.append(line)

    return "\n".join(cleaned)


def run_test(binary, infile, outfile):

    proc = subprocess.run(
        [binary, str(infile)],
        capture_output=True,
        text=True
    )

    actual = normalize(proc.stdout)

    expected = normalize(Path(outfile).read_text())

    if actual == expected:
        print(f"PASS {infile.name}")
        return True

    print(f"FAIL {infile.name}")

    diff = difflib.unified_diff(
        expected.splitlines(),
        actual.splitlines(),
        fromfile="expected",
        tofile="actual",
        lineterm=""
    )

    for line in diff:
        print(line)

    return False


def main():

    if len(sys.argv) != 3:
        print("Usage: python test_runner.py <scheduler_binary> <test_dir>")
        sys.exit(1)

    binary = sys.argv[1]
    test_dir = Path(sys.argv[2])

    passed = 0
    total = 0

    for infile in sorted(test_dir.glob("*.in")):
        outfile = infile.with_suffix(".out")

        if not outfile.exists():
            print(f"Missing expected output for {infile}")
            continue

        total += 1

        if run_test(binary, infile, outfile):
            passed += 1

    print()
    print(f"{passed}/{total} tests passed")


if __name__ == "__main__":
    main()
