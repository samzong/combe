import os
import plistlib
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TAG = re.compile(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-(alpha|beta|rc)\.(0|[1-9][0-9]*))?")


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def build_number(value):
    if not re.fullmatch(r"(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*)){0,2}", value):
        raise ValueError("CFBundleVersion must contain one to three integers")
    parts = tuple(map(int, value.split(".")))
    return parts + (0,) * (3 - len(parts))


def main():
    mode, tag = sys.argv[1:]
    if mode not in {"validate", "prepare"} or not TAG.fullmatch(tag):
        raise ValueError("Expected validate or prepare and a strict SemVer release tag")
    if mode == "prepare" and os.environ.get("DRY_RUN") == "true":
        return
    version = tag[1:]
    core = version.split("-")[0]
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    if manifest != version:
        raise ValueError(f"Release tag {tag} does not match Cargo.toml version {manifest}")
    path = ROOT / "packaging/Info.plist"
    plist = plistlib.loads(path.read_bytes())
    if mode == "prepare":
        build = str(build_number(plist["CFBundleVersion"])[0] + 1)
        content = path.read_text()
        for key, value in (("CFBundleShortVersionString", core), ("CFBundleVersion", build)):
            content, count = re.subn(rf"(<key>{key}</key>\s*<string>)[^<]*(</string>)", rf"\g<1>{value}\g<2>", content)
            if count != 1:
                raise ValueError(f"Expected one {key}")
        path.write_text(content)
        previous_stable = git("describe", "--tags", "--abbrev=0", "--match", "v[0-9]*", "--exclude", "*-*")
        subprocess.run(["git-cliff", f"{previous_stable}..HEAD", "--ignore-tags", r".*-(alpha|beta|rc)\.[0-9]+$", "--tag", tag, "--prepend", "CHANGELOG.md"], cwd=ROOT, check=True)
        changelog = ROOT / "CHANGELOG.md"
        changelog.write_text(changelog.read_text().strip() + "\n")
    else:
        if plist["CFBundleShortVersionString"] != core:
            raise ValueError(f"Info.plist short version does not match release core {core}")
        current = build_number(plist["CFBundleVersion"])
        previous_tag = git("describe", "--tags", "--abbrev=0", "--match", "v[0-9]*", "HEAD^")
        previous = plistlib.loads(git("show", f"{previous_tag}:packaging/Info.plist").encode())
        if tag != previous_tag and current <= build_number(previous["CFBundleVersion"]):
            raise ValueError(f"CFBundleVersion must increase after {previous_tag}")
    print(f"Release version matches: {tag}")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, subprocess.CalledProcessError) as error:
        sys.exit(f"error: {error}")
