"""Package a desktop build of the viewer (docs/client.md 8).

Usage: python scripts/package.py BINARY OUTDIR --platform windows|macos|linux [--version V]

BINARY  the built viewer (acs-viewer or acs-viewer.exe)
OUTDIR  where the package goes: acs-viewer-V-windows-x64.zip,
        acs-viewer-V-macos-universal.zip (holding acs-viewer.app) or
        acs-viewer-V-linux-x64.tar.gz

Each holds the program, LICENSE, LICENSES/ and a README.txt. The project's
name comes from the workspace's description in Cargo.toml, never from
here. V defaults to the workspace version.
"""
import argparse, io, os, re, stat, tarfile, zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
NAMES = {"windows": "windows-x64", "macos": "macos-universal", "linux": "linux-x64"}
NOTES = {
    "windows": "Start acs-viewer.exe. Windows may warn that the publisher is unknown:\n"
               "choose More info, then Run anyway.",
    "macos": "The app is not signed. The first time, right-click acs-viewer.app, choose\n"
             "Open, then Open again; after that it opens as usual.",
    "linux": "Start ./acs-viewer. It needs X11 or Wayland, OpenGL and ALSA (on Debian\n"
             "and Ubuntu: libgl1, libxkbcommon0 and libasound2).",
}


def workspace():
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    pkg = text.split("[workspace.package]", 1)[1]
    version = re.search(r'^version\s*=\s*"([^"]+)"', pkg, re.M).group(1)
    description = re.search(r'^description\s*=\s*"([^"]+)"', pkg, re.M).group(1)
    return version, description


def readme(description, version, platform):
    return f"""{description}
Version {version}, {NAMES[platform]}.

{NOTES[platform]}

The title screen offers First Voyage, a short tutorial adventure of our
own, built in. To play another, open an adventure you own (a C64 .d64 disk
image, a PC ADVEN.FPY or ADVEN.HRD, or an .advx bundle) with Open..., or
drop it on the window. No adventure of EA's comes with this program, and
the files you open are read on your computer only.

The program is free software under the GNU GPL, version 3 or later
(LICENSE). Its documents are under the Creative Commons licenses in
LICENSES/. This project is not affiliated with or endorsed by Electronic
Arts.
"""


def info_plist(version):
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>acs-viewer</string>
  <key>CFBundleIdentifier</key><string>io.github.dragynrain.acs-viewer</string>
  <key>CFBundleName</key><string>acs-viewer</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>{version}</string>
  <key>CFBundleVersion</key><string>{version}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
"""


def entries(binary, platform, version, description):
    """(path in the package, bytes, executable) for every file."""
    exe = binary.read_bytes()
    files = []
    if platform == "macos":
        app = "acs-viewer.app/Contents"
        files.append((f"{app}/MacOS/acs-viewer", exe, True))
        files.append((f"{app}/Info.plist", info_plist(version).encode(), False))
    else:
        files.append(("acs-viewer.exe" if platform == "windows" else "acs-viewer", exe, True))
    files.append(("README.txt", readme(description, version, platform).encode(), False))
    files.append(("LICENSE", (ROOT / "LICENSE").read_bytes(), False))
    for f in sorted((ROOT / "LICENSES").iterdir()):
        files.append((f"LICENSES/{f.name}", f.read_bytes(), False))
    return files


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("binary", type=Path)
    ap.add_argument("outdir", type=Path)
    ap.add_argument("--platform", choices=sorted(NAMES), required=True)
    ap.add_argument("--version")
    args = ap.parse_args()
    version, description = workspace()
    version = args.version or version
    base = f"acs-viewer-{version}-{NAMES[args.platform]}"
    files = entries(args.binary, args.platform, version, description)
    args.outdir.mkdir(parents=True, exist_ok=True)
    if args.platform == "linux":
        out = args.outdir / f"{base}.tar.gz"
        with tarfile.open(out, "w:gz") as t:
            for name, data, executable in files:
                info = tarfile.TarInfo(f"{base}/{name}")
                info.size = len(data)
                info.mode = 0o755 if executable else 0o644
                t.addfile(info, io.BytesIO(data))
    else:
        out = args.outdir / f"{base}.zip"
        with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
            for name, data, executable in files:
                info = zipfile.ZipInfo(f"{base}/{name}", date_time=(2026, 1, 1, 0, 0, 0))
                info.compress_type = zipfile.ZIP_DEFLATED
                # Unix permissions, so the app's program stays executable.
                mode = 0o755 if executable else 0o644
                info.external_attr = (stat.S_IFREG | mode) << 16
                z.writestr(info, data)
    print(f"{out} ({os.path.getsize(out) // 1024} KB, {len(files)} files)")


if __name__ == "__main__":
    main()
