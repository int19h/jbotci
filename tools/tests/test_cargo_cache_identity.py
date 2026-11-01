"""Bind target caches to the compiler, image, OS, and installed packages."""
import hashlib
import io
import json
import unittest
from unittest.mock import patch

from tools import cargo_cache_identity as c


class CacheIdentityTests(unittest.TestCase):
    def identity(self, rustc="rustc full version\n", packages="clang\t1\tarm64\n", image="sha256:" + "a" * 64, system="Linux"):
        with patch.object(c.subprocess, "check_output", side_effect=[rustc, packages]), patch.dict(c.os.environ, RUNNER_OS=system):
            return c.identity(image)

    def test_every_environment_input_changes_the_key(self):
        original = self.identity()
        for changed in (self.identity(rustc="different compiler\n"), self.identity(packages="clang\t2\tarm64\n"),
                        self.identity(image="sha256:" + "b" * 64), self.identity(system="Windows")):
            self.assertNotEqual(original["key"], changed["key"])
        self.assertEqual(original["rustc_vv"], "rustc full version\n")

    def test_package_order_does_not_change_the_key(self):
        self.assertEqual(self.identity(packages="a\nb\n"), self.identity(packages="b\na\n"))

    def test_mutable_image_is_rejected(self):
        with self.assertRaises(ValueError):
            c.identity("rust:1-bookworm")

    def test_registry_digest_is_compared_with_manifest_bytes(self):
        body = b'{"manifests":[]}'
        token = io.BytesIO(json.dumps({"token": "secret"}).encode())
        manifest = io.BytesIO(body)
        digest = "sha256:" + hashlib.sha256(body).hexdigest()
        manifest.headers = {"Docker-Content-Digest": digest}
        with patch.object(c.urllib.request, "urlopen", side_effect=[token, manifest]):
            self.assertEqual(c.resolve_image("rust:1-bookworm"), ("rust:1-bookworm@" + digest, digest))
        token = io.BytesIO(b'{"token":"secret"}')
        manifest = io.BytesIO(body)
        manifest.headers = {"Docker-Content-Digest": "sha256:" + "0" * 64}
        with patch.object(c.urllib.request, "urlopen", side_effect=[token, manifest]), self.assertRaises(ValueError):
            c.resolve_image("rust:1-bookworm")
