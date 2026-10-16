# SPDX-License-Identifier: GPL-3.0-or-later
"""Publication gate regression tests. Inputs are synthetic; no device data is used."""
import importlib.util
import pathlib
import struct
import unittest

MODULE_PATH = pathlib.Path(__file__).resolve().parents[1] / "check_repository.py"
SPEC = importlib.util.spec_from_file_location("repository_gate", MODULE_PATH)
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)


class PublicationGateTests(unittest.TestCase):
    def test_device_literals_are_blocked_outside_narrow_fixtures(self):
        value = ":".join(["12", "34", "56", "78", "9A", "BC"])
        self.assertIn("device-address literal", GATE.text_issues("docs/example.md", value))
        sequence = ":".join(f"{i:02X}" for i in range(1, 7))
        self.assertEqual([], GATE.text_issues("crates/realme-buds/tests/protocol.rs", sequence))
        self.assertTrue(GATE.text_issues("README.md", sequence))

    def test_credential_and_home_path_patterns_are_blocked(self):
        token = "ghp" + "_" + "x" * 40
        home = "C:" + "/" + "Users" + "/" + "example" + "/file"
        self.assertIn("credential/private-key pattern", GATE.text_issues("sample.rs", token))
        self.assertIn("developer home-directory path", GATE.text_issues("sample.rs", home))

    def test_native_sources_are_audited_and_binaries_are_not_committable(self):
        self.assertTrue({".h", ".hpp", ".c", ".cpp"}.issubset(GATE.TEXT_SUFFIXES))
        self.assertTrue({".dll", ".so", ".dylib", ".lib", ".zip"}.issubset(GATE.FORBIDDEN_SUFFIXES))

    def test_png_text_metadata_is_rejected(self):
        data = b"\x89PNG\r\n\x1a\n" + struct.pack(">I", 0) + b"tEXt" + b"\x00" * 4
        self.assertTrue(GATE.png_issues(data))


if __name__ == "__main__":
    unittest.main()
