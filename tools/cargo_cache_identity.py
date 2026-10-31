"""Resolve immutable CI images and identify the installed build environment."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import urllib.parse
import urllib.request


def resolve_image(tag: str) -> tuple[str, str]:
    # CI uses one official Docker image. Refuse an unrecognized registry path.
    if not re.fullmatch(r"rust:[A-Za-z0-9._-]+", tag):
        raise ValueError("Only an official rust image tag is supported")
    token_url = "https://auth.docker.io/token?" + urllib.parse.urlencode({
        "service": "registry.docker.io", "scope": "repository:library/rust:pull"})
    with urllib.request.urlopen(token_url, timeout=60) as response:
        token = json.load(response)["token"]
    request = urllib.request.Request(
        "https://registry-1.docker.io/v2/library/rust/manifests/" + tag.split(":", 1)[1],
        headers={"Authorization": "Bearer " + token,
                 "Accept": "application/vnd.oci.image.index.v1+json, application/vnd.docker.distribution.manifest.list.v2+json"})
    with urllib.request.urlopen(request, timeout=60) as response:
        body = response.read()
        digest = response.headers["Docker-Content-Digest"]
    if digest != "sha256:" + hashlib.sha256(body).hexdigest():
        raise ValueError("The image manifest does not match its registry digest")
    return tag + "@" + digest, digest


def identity(image_digest: str) -> dict[str, str]:
    if image_digest != "none" and not re.fullmatch(r"sha256:[0-9a-f]{64}", image_digest):
        raise ValueError("The cache needs an immutable image digest")
    rustc = subprocess.check_output(["rustc", "-vV"], text=True)
    packages = subprocess.check_output([
        "dpkg-query", "-W", "-f=${binary:Package}\t${Version}\t${Architecture}\n"], text=True)
    packages = "\n".join(sorted(packages.splitlines())) + "\n"
    value = {"rustc_vv": rustc, "runner_os": os.environ.get("RUNNER_OS", platform.system()),
             "image_digest": image_digest, "system_packages_sha256": hashlib.sha256(packages.encode()).hexdigest()}
    value["key"] = hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()
    return value


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("resolve-image", "identity"))
    parser.add_argument("value")
    arguments = parser.parse_args()
    if arguments.mode == "resolve-image":
        image, digest = resolve_image(arguments.value)
        result = {"image": image, "digest": digest}
    else:
        result = identity(arguments.value)
    if "GITHUB_OUTPUT" in os.environ:
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            for key in ("image", "digest", "key"):
                if key in result:
                    output.write(f"{key}={result[key]}\n")
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
