from __future__ import annotations

import contextlib
import copy
import importlib.util
import io
import json
import sys
import tempfile
import types
import unittest
from pathlib import Path
from unittest import mock

import numpy as np


SCRIPT = Path(__file__).resolve().parents[1] / 'embedding-pack/f2llm/build-vector-pack.py'
SPEC = importlib.util.spec_from_file_location('vector_pack', SCRIPT)
pack = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(pack)


class Tokenizer:
    eos_token_id = 1
    pad_token_id = 0
    special_tokens_map = {'eos_token': '<eos>'}
    backend_tokenizer = types.SimpleNamespace(to_str=lambda: 'test-tokenizer-v1')

    def __call__(self, text, **kwargs):
        return {'input_ids': [ord(c) for c in text]}


class Session:
    def __init__(self, *args, **kwargs):
        pass

    def get_inputs(self):
        return [types.SimpleNamespace(name=name) for name in ('input_ids', 'attention_mask', 'position_ids')]

    def run(self, outputs, feeds):
        ids = feeds['input_ids']
        # Each window depends on all of its tokens, independent of batch padding.
        sums = (ids * feeds['attention_mask']).sum(axis=1)
        hidden = np.zeros((*ids.shape, 3), dtype=np.float32)
        hidden[:, :, 0] = sums[:, None] % 17 + 1
        hidden[:, :, 1] = sums[:, None] % 23 + 2
        hidden[:, :, 2] = sums[:, None] % 31 + 3
        return [hidden]


def document(index, text):
    return {'id': index, 'input': text, 'inputHash': pack.sha256(text.encode()), 'kind': 'test'}


class IncrementalEmbeddingPacksTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.model = self.root / 'model.onnx'
        self.model.write_bytes(b'test-model')
        self.modules = mock.patch.dict(sys.modules, {
            'onnxruntime': types.SimpleNamespace(SessionOptions=types.SimpleNamespace, InferenceSession=Session),
            'transformers': types.SimpleNamespace(AutoTokenizer=types.SimpleNamespace(from_pretrained=lambda *a, **k: Tokenizer())),
            'onnx': types.SimpleNamespace(load=lambda path, **k: types.SimpleNamespace(SerializeToString=lambda **kwargs: Path(path).read_bytes())),
        })
        self.modules.start()
        self.addCleanup(self.modules.stop)
        versions = mock.patch.object(pack, 'version', return_value='test-version')
        versions.start()
        self.addCleanup(versions.stop)
        self.old = {'dictionary': [document(0, 'keep long enough for several windows'), document(1, 'change'), document(2, 'remove')],
                    'cll': [document(0, 'CLL keep'), document(1, 'another CLL windowed document')]}
        self.new = {'dictionary': [document(0, 'changed input'), document(1, 'keep long enough for several windows'), document(2, 'added')],
                    'cll': copy.deepcopy(self.old['cll'])}

    def build(self, name, rows, *extra):
        corpus = dict(rows, inputFormatVersion='test-v1', inputHash=pack.sha256(pack.json_bytes(rows)), dictionaryHash='dict-hash', cllHash='cll-hash')
        corpus_path = self.root / f'{name}-input.json'
        pack.write_json(corpus_path, corpus)
        output = self.root / name
        args = ['build-vector-pack.py', '--input', str(corpus_path), '--out', str(output), '--q4-onnx', str(self.model),
                '--dimensions', '3', '--max-sequence-length', '8', '--batch-size', '2', *extra]
        with mock.patch.object(sys, 'argv', args), contextlib.redirect_stdout(io.StringIO()) as stdout, mock.patch.object(pack, 'embed_texts', wraps=pack.embed_texts) as embed:
            pack.main()
        manifest = next(output.rglob('manifest.json'))
        return output, manifest, embed, stdout.getvalue()

    def test_incremental_matches_full_and_only_embeds_changed_and_added_rows(self):
        previous, manifest, _, _ = self.build('previous', self.old)
        full, _, _, _ = self.build('full', self.new)
        incremental, _, embed, output = self.build('incremental', self.new, '--reuse-from', str(manifest.parent))
        self.assertEqual(embed.call_args_list[0].args[0], ['changed input', 'added'])
        self.assertEqual(embed.call_args_list[1].args[0], [])
        self.assertIn('vlacku-en: reused 1 rows, embedded 2 rows', output)
        self.assertIn('cukta-cll: reused 2 rows, embedded 0 rows', output)
        def contents(root):
            return {str(p.relative_to(root)): p.read_bytes() for p in root.rglob('*') if p.is_file()}
        self.assertEqual(contents(full), contents(incremental))
        # Catalog selection must take the same path as a direct manifest source.
        from_catalog, _, _, _ = self.build('catalog-reuse', self.new, '--reuse-catalog', str(previous / 'catalog.json'))
        self.assertEqual(contents(full), contents(from_catalog))

    def test_incompatible_and_legacy_sources_are_refused(self):
        _, path, _, _ = self.build('previous', self.old)
        original = pack.read_json(path)
        for field, value in [('model_key', 'other'), ('q4_onnx_sha256', 'other'), ('max_window_tokens', 99),
                             ('dimensions', 4), ('element_type', 'f32le'), ('pooling', 'other'),
                             ('vector_space_key', 'other'), ('tokenizer_sha256', 'other'), ('loaded_onnx_sha256', 'other')]:
            with self.subTest(field=field):
                manifest = copy.deepcopy(original)
                manifest['reuse_identity'][field] = value
                pack.write_json(path, manifest)
                with self.assertRaisesRegex(ValueError, 'incompatible reuse pack'):
                    self.build('refused', self.new, '--reuse-from', str(path))
        original.pop('reuse_identity')
        pack.write_json(path, original)
        with self.assertRaisesRegex(ValueError, 'legacy pack'):
            self.build('legacy', self.new, '--reuse-from', str(path))

    def test_top_level_manifest_disagreement_is_refused(self):
        _, path, _, _ = self.build('previous', self.old)
        manifest = pack.read_json(path)
        manifest['element_type'] = 'f32le'
        pack.write_json(path, manifest)
        with self.assertRaisesRegex(ValueError, 'element_type differs'):
            self.build('refused', self.new, '--reuse-from', str(path))

    def test_local_and_downloaded_corruption_is_refused_before_embedding(self):
        _, path, _, _ = self.build('previous', self.old)
        manifest = pack.read_json(path)
        for kind in ['items', 'vector']:
            with self.subTest(kind=kind):
                child = path.parent / manifest['corpora'][0][f'{kind}_url']
                original = child.read_bytes()
                child.write_bytes(original + b'corrupt')
                with self.assertRaisesRegex(ValueError, 'SHA-256 mismatch'):
                    self.build(f'bad-{kind}', self.new, '--reuse-from', str(path))
                # Exercise the URL reader without an external server or any model download.
                def download(url, **kwargs):
                    relative = url.removeprefix('https://example.test/pack/')
                    return io.BytesIO((path.parent / relative).read_bytes())
                with mock.patch.object(pack, 'urlopen', side_effect=download), self.assertRaisesRegex(ValueError, 'SHA-256 mismatch'):
                    self.build(f'bad-url-{kind}', self.new, '--reuse-from', 'https://example.test/pack/manifest.json')
                child.write_bytes(original)

    def test_overwrite_remains_opt_in(self):
        self.build('existing', self.old)
        with self.assertRaisesRegex(SystemExit, 'already exists'):
            self.build('existing', self.new)

    def test_untrusted_input_hash_is_refused(self):
        rows = copy.deepcopy(self.new)
        rows['dictionary'][0]['inputHash'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'exact input text'):
            self.build('bad-input', rows)


if __name__ == '__main__':
    unittest.main()
