#!/usr/bin/env python3
"""Télécharge ONNX Runtime 1.31.1 depuis PyPI et le dépose dans dist/<plateforme>/.

Le runtime est la bibliothèque sur laquelle tourne l'embedding (`embedding/e5-small`). Il n'est pas
commité : chaque build le retire du wheel officiel, et refuse tout fichier dont le hash diffère des
valeurs épinglées ici (le wheel entier, puis la bibliothèque extraite).

Usage : python3 tools/runtime/fetch_onnxruntime.py linux dist/linux-amd64
        python3 tools/runtime/fetch_onnxruntime.py windows dist/windows-amd64
"""

import hashlib
import io
import json
import os
import sys
import urllib.request
import zipfile

VERSION = "1.31.1"

# Pins vérifiés contre les wheels PyPI (cp312) et le RECORD du wheel Windows.
PINS = {
    "linux": {
        "wheel": "onnxruntime-1.31.1-cp312-cp312-manylinux_2_28_x86_64.whl",
        "wheel_sha256": "1f3dea47f2f0e5117c2404cea7d21fd253ae7836a3bf167a501cdfbb2600d536",
        "member": "onnxruntime/capi/libonnxruntime.so.1.31.1",
        "lib_sha256": "fc44112fbe06d7d689c57e11d3862031f64711007e4ee020f8eed5185fd2c1d3",
        "out": "libonnxruntime.so",
    },
    "windows": {
        "wheel": "onnxruntime-1.31.1-cp312-cp312-win_amd64.whl",
        "wheel_sha256": "ae8a9a351efda685cc59a66226cdefaa37a126c1d66872ca23a2de9425326130",
        "member": "onnxruntime/capi/onnxruntime.dll",
        "lib_sha256": "e45f61e50d8650885452be538a00bb14a715e46a1bc3691000ce947e89ed0306",
        "out": "onnxruntime.dll",
    },
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def wheel_url(filename: str) -> str:
    """URL du wheel selon l'index JSON de PyPI, pour la version épinglée."""
    index = json.load(urllib.request.urlopen(f"https://pypi.org/pypi/onnxruntime/{VERSION}/json"))
    for entry in index["urls"]:
        if entry["filename"] == filename:
            return entry["url"]
    raise SystemExit(f"wheel absent de PyPI : {filename}")


def main() -> int:
    if len(sys.argv) != 3 or sys.argv[1] not in PINS:
        print(__doc__, file=sys.stderr)
        return 2
    pin = PINS[sys.argv[1]]
    dest_dir = sys.argv[2]

    blob = urllib.request.urlopen(wheel_url(pin["wheel"])).read()
    if sha256(blob) != pin["wheel_sha256"]:
        raise SystemExit("hash du wheel incorrect : refus")
    library = zipfile.ZipFile(io.BytesIO(blob)).read(pin["member"])
    if sha256(library) != pin["lib_sha256"]:
        raise SystemExit("hash de la bibliothèque incorrect : refus")

    os.makedirs(dest_dir, exist_ok=True)
    target = os.path.join(dest_dir, pin["out"])
    with open(target, "wb") as handle:
        handle.write(library)
    print(f"runtime prêt : {target} ({len(library)} octets, sha256 {pin['lib_sha256']})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
