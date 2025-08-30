import subprocess
import re


def gen_ci_script():
    cmd = ["maturin", "generate-ci", "github"]
    return subprocess.run(cmd, capture_output=True).stdout.decode()


def amend_env(file):
    pattern = r"^(\s*runs-on:.*)$"
    replacement = r"""\1
    env:
      CFLAGS_s390x_unknown_linux_gnu: '-march=z10'"""
    return re.sub(pattern, replacement, file, flags=re.MULTILINE)


def generate_ci(path=".github/workflows/python_ci.yaml"):
    ci = gen_ci_script()
    ci = amend_env(ci)
    with open(path, "w") as file:
        file.write(ci)


def main():
    generate_ci()


if __name__ == "__main__":
    main()
