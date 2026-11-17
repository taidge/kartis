"""Exercise release gates without uploading a package or reading credentials."""

from contextlib import redirect_stdout
from io import BytesIO, StringIO
import hashlib
import json
from pathlib import Path
import tempfile
import tarfile
import unittest
from unittest.mock import patch
import urllib.error

import release


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "Cargo.toml").write_text('[package]\nname="kartis"\nversion="0.2.0"\n')
        self.target = self.root / "target"
        self.crate = self.target / "package" / "kartis-0.2.0.crate"
        self.crate.parent.mkdir(parents=True)
        self.write_crate()
        self.checksum = hashlib.sha256(self.crate.read_bytes()).hexdigest()
        self.dirty = ""
        self.tagged = "abc"
        self.url_error = None
        self.published = {"checksum": self.checksum, "yanked": False}
        self.stack = []
        for context in [
            patch.object(release, "ROOT", self.root),
            patch.object(release, "command", side_effect=self.command),
            patch.object(release.urllib.request, "urlopen", side_effect=self.urlopen),
            patch.dict(release.os.environ, {}, clear=True),
            redirect_stdout(StringIO()),
        ]:
            context.__enter__()
            self.addCleanup(context.__exit__, None, None, None)

    def command(self, *args):
        if args[:2] == ("cargo", "metadata"):
            return json.dumps({"target_directory": str(self.target)})
        if args[:2] == ("git", "status"):
            return self.dirty
        return "abc" if args[-1] == "HEAD" else self.tagged

    def write_crate(self, commit="abc", dirty=False):
        data = json.dumps({"git": {"sha1": commit, "dirty": dirty}, "path_in_vcs": ""}).encode()
        with tarfile.open(self.crate, "w:gz") as archive:
            entry = tarfile.TarInfo("kartis-0.2.0/.cargo_vcs_info.json")
            entry.size = len(data)
            archive.addfile(entry, BytesIO(data))

    def urlopen(self, request, **kwargs):
        self.assertEqual(request.full_url, "https://crates.io/api/v1/crates/kartis/0.2.0")
        self.assertEqual(request.get_header("User-agent"), "kartis-release-check/0.2.0")
        if self.url_error:
            raise self.url_error
        return BytesIO(json.dumps({"version": self.published}).encode())

    def test_published_checksum_and_output_paths(self):
        output = self.root / "outputs"
        with patch.dict(release.os.environ, {"GITHUB_OUTPUT": str(output)}):
            release.verify("v0.2.0", True)
        self.assertIn("exists=true\n", output.read_text())
        self.assertIn(f"crate_path={self.crate}\n", output.read_text())
        self.assertIn(f"checksum_path={self.crate.parent / 'SHA256SUMS'}\n", output.read_text())
        self.assertEqual((self.crate.parent / "SHA256SUMS").read_text(), f"{self.checksum}  {self.crate.name}\n")
        self.assertEqual((self.crate.parent / "SHA256SUMS").read_bytes(), f"{self.checksum}  {self.crate.name}\n".encode())

    def test_wrong_tag_version(self):
        with self.assertRaisesRegex(ValueError, "must match"):
            release.verify("v0.3.0")

    def test_tag_must_point_to_checkout(self):
        self.tagged = "other"
        with self.assertRaisesRegex(ValueError, "checked-out"):
            release.verify("v0.2.0")

    def test_dirty_checkout_rejected(self):
        self.dirty = " M Cargo.toml"
        with self.assertRaisesRegex(ValueError, "clean"):
            release.verify("v0.2.0")

    def test_stale_package_rejected(self):
        self.write_crate(commit="previous-commit")
        with self.assertRaisesRegex(ValueError, "clean release commit"):
            release.verify("v0.2.0")

    def test_dirty_package_rejected(self):
        self.write_crate(dirty=True)
        with self.assertRaisesRegex(ValueError, "clean release commit"):
            release.verify("v0.2.0")

    def test_missing_version_allowed_before_publish(self):
        self.url_error = urllib.error.HTTPError("https://crates.io", 404, "missing", {}, None)
        release.verify("v0.2.0")

    def test_missing_version_rejected_after_publish(self):
        self.url_error = urllib.error.HTTPError("https://crates.io", 404, "missing", {}, None)
        with self.assertRaisesRegex(ValueError, "not published"):
            release.verify("v0.2.0", True)

    def test_network_error_is_not_an_absent_version(self):
        self.url_error = urllib.error.HTTPError("https://crates.io", 503, "unavailable", {}, None)
        with self.assertRaises(urllib.error.HTTPError):
            release.verify("v0.2.0")

    def test_checksum_mismatch_rejected(self):
        self.published["checksum"] = "other"
        with self.assertRaisesRegex(ValueError, "checksum"):
            release.verify("v0.2.0")

    def test_yanked_version_rejected(self):
        self.published["yanked"] = True
        with self.assertRaisesRegex(ValueError, "yanked"):
            release.verify("v0.2.0")


if __name__ == "__main__":
    unittest.main()
