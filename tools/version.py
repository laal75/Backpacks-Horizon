"""The mod's version, kept in VERSION and copied everywhere else that names it.

    python tools/version.py            check that every file agrees with VERSION
    python tools/version.py 0.3.0      set a new version: VERSION, both manifests,
                                       the content (regenerated), and CHANGELOG.md's
                                       "Unreleased" section becomes "0.3.0 - <today>"

Versions are semver (MAJOR.MINOR.PATCH): PATCH for fixes, MINOR for new blocks or features,
MAJOR when a world saved with the old version would lose or change something. Tag each release
`v<version>` and attach dist/'s files to its GitHub release.
"""
import datetime
import os
import re
import subprocess
import sys

MOD = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VERSION_FILE = os.path.join(MOD, "VERSION")
# The files that carry the version, each on its first `version = "..."` line.
FILES = ["content/horizon-mod.toml", "native/horizon-mixin.toml"]
LINE = re.compile(r'^version\s*=\s*"([^"]*)"', re.M)
SEMVER = re.compile(r"^\d+\.\d+\.\d+$")


def read_version():
    with open(VERSION_FILE, encoding="utf-8") as f:
        return f.read().strip()


def found(rel):
    with open(os.path.join(MOD, rel), encoding="utf-8") as f:
        return LINE.search(f.read()).group(1)


def check():
    version = read_version()
    wrong = [(rel, v) for rel in FILES if (v := found(rel)) != version]
    for rel, v in wrong:
        print(f"{rel}: {v}, not {version}")
    return version, not wrong


def set_version(version):
    if not SEMVER.match(version):
        sys.exit(f"{version} is not MAJOR.MINOR.PATCH")
    with open(VERSION_FILE, "w", encoding="utf-8", newline="\n") as f:
        f.write(version + "\n")
    for rel in FILES:
        path = os.path.join(MOD, rel)
        with open(path, encoding="utf-8") as f:
            text = f.read()
        text = LINE.sub(f'version = "{version}"', text, count=1)
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
    # The content's horizon-mod.toml is the generator's.
    subprocess.run([sys.executable, os.path.join(MOD, "tools", "gen_content.py"), os.path.join(MOD, "content")],
                   check=True, stdout=subprocess.DEVNULL)
    changelog = os.path.join(MOD, "CHANGELOG.md")
    with open(changelog, encoding="utf-8") as f:
        text = f.read()
    today = datetime.date.today().isoformat()
    if "## Unreleased" in text:
        text = text.replace("## Unreleased", f"## Unreleased\n\n## {version} - {today}", 1)
        with open(changelog, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
    print(f"{version}: now commit, then `git tag v{version}` and `python tools/pack.py`")


if __name__ == "__main__":
    if len(sys.argv) > 1:
        set_version(sys.argv[1])
    version, ok = check()
    print(version if ok else "versions disagree")
    sys.exit(0 if ok else 1)
