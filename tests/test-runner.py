# John Vezzola, 3/10/26
# ---------------------
# Simple Test Runner.
# 
# USAGE: python3 test-runner.py [program.exe] [test1 test2 ...]
#
# The arguments to stdin should be the executable followed by test files to run in order.
# It is expected that a x.in and x.out exists for each argument x.
# 
# ---------------------

from pathlib import Path
import subprocess
import sys

def main(argv: list[str]|None = None):

    # Resolve each argument to a real filepath, raise OSError if a file doesn't exist.
    test_exe = Path(argv[1]).resolve(strict=True)

    # Files are stored in tuple pairs of [(input1, solution1), (input2, solution2)...]
    test_files = []
    for file in argv[2:]:
        input_file = Path(file + ".in").resolve(strict=True)
        solution_file = Path(file + ".out").resolve(strict=True)
        test_files.append((input_file, solution_file))

    print(test_files)





if __name__ == "__main__":
    main(sys.argv)