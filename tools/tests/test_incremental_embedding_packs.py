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

try:
    import onnx as real_onnx
except ImportError:
    real_onnx = None


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
            'onnx': types.SimpleNamespace(load=lambda path, **k: types.SimpleNamespace(DESCRIPTOR=types.SimpleNamespace(full_name='onnx.ModelProto'), ListFields=lambda: [])),
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
                             ('vector_space_key', 'other'), ('tokenizer_sha256', 'other'), ('external_data_sha256', {'weights': 'other'})]:
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

    def test_equal_input_can_move_between_corpora(self):
        _, previous, _, _ = self.build('previous', self.old)
        rows = {'dictionary': [document(0, self.old['cll'][0]['input'])], 'cll': []}
        full, _, _, _ = self.build('full', rows)
        incremental, _, embed, _ = self.build('incremental', rows, '--reuse-from', str(previous))
        self.assertEqual([call.args[0] for call in embed.call_args_list], [[], []])
        for path in full.rglob('*'):
            if path.is_file():
                self.assertEqual(path.read_bytes(), (incremental / path.relative_to(full)).read_bytes())

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
                    self.assertEqual(url.get_header('User-agent'), 'jbotci-embedding-pack-builder/1')
                    relative = url.full_url.removeprefix('https://example.test/pack/')
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

    def test_explicit_empty_source_does_not_fall_back_to_full_embedding(self):
        for flag in ['--reuse-from', '--reuse-catalog']:
            with self.subTest(flag=flag), self.assertRaisesRegex(ValueError, 'reuse source must not be empty'):
                self.build('empty-source', self.new, flag, '')

    def test_backend_changes_do_not_block_reuse_but_model_and_tokenizer_do(self):
        _, path, _, _ = self.build('previous', self.old)
        with mock.patch.object(pack, 'version', return_value='new-backend'):
            _, _, embed, _ = self.build('new-backend', self.old, '--reuse-from', str(path))
        self.assertEqual([call.args[0] for call in embed.call_args_list], [[], []])
        self.model.write_bytes(b'changed-model-bytes')
        with self.assertRaisesRegex(ValueError, 'q4_onnx_sha256'):
            self.build('changed-model', self.old, '--reuse-from', str(path), '--adopt-legacy-pack')
        self.model.write_bytes(b'test-model')
        with mock.patch.object(Tokenizer.backend_tokenizer, 'to_str', return_value='changed-tokenizer'):
            with self.assertRaisesRegex(ValueError, 'tokenizer_sha256'):
                self.build('changed-tokenizer', self.old, '--reuse-from', str(path), '--adopt-legacy-pack')

    def test_legacy_adoption_requires_all_recorded_fields_to_match(self):
        _, path, _, _ = self.build('previous', self.old)
        original = pack.read_json(path)
        original.pop('reuse_identity')
        original.pop('build_provenance')
        pack.write_json(path, original)
        _, new_path, embed, _ = self.build('adopted', self.old, '--reuse-from', str(path), '--adopt-legacy-pack')
        self.assertEqual([call.args[0] for call in embed.call_args_list], [[], []])
        self.assertEqual(pack.read_json(new_path)['legacy_adoption']['compatibility'], 'owner-attested-unrecorded-identity')
        _, descendant, _, _ = self.build('descendant', self.old, '--reuse-from', str(new_path))
        self.assertEqual(pack.read_json(descendant)['legacy_adoption'], pack.read_json(new_path)['legacy_adoption'])
        for field in ('model_key', 'web_model', 'model_revision', 'q4_onnx_sha256', 'vector_space_key',
                      'max_sequence_length', 'max_window_tokens', 'pooling', 'dimensions', 'element_type', 'normalized', 'distance'):
            with self.subTest(field=field):
                bad = copy.deepcopy(original)
                bad[field] = 'mismatch'
                pack.write_json(path, bad)
                with self.assertRaisesRegex(ValueError, 'incompatible reuse pack'):
                    self.build('bad-legacy', self.old, '--reuse-from', str(path), '--adopt-legacy-pack')
        # Adoption does not weaken the per-row input hash requirement.
        pack.write_json(path, original)
        child = path.parent / original['corpora'][0]['items_url']
        items = pack.read_json(child)
        items[0]['input_hash'] = 'invalid'
        pack.write_json(child, items)
        original['corpora'][0]['items_sha256'] = pack.file_sha256(child)
        pack.write_json(path, original)
        with self.assertRaisesRegex(ValueError, 'input hash'):
            self.build('bad-row-hash', self.old, '--reuse-from', str(path), '--adopt-legacy-pack')

    def test_missing_and_duplicate_corpora_are_refused(self):
        _, path, _, _ = self.build('previous', self.old)
        original = pack.read_json(path)
        for corpora in (original['corpora'][:1], [original['corpora'][0]] * 2):
            manifest = dict(original, corpora=corpora)
            pack.write_json(path, manifest)
            with self.assertRaisesRegex(ValueError, 'both corpora|duplicate reuse corpus'):
                self.build('bad-corpora', self.old, '--reuse-from', str(path))

    def test_conflicting_stored_bytes_include_signed_zero(self):
        rows = {'dictionary': [document(0, 'same')], 'cll': [document(0, 'same')]}
        _, path, _, _ = self.build('previous', rows)
        original = pack.read_json(path)
        for index, corpus in enumerate(original['corpora']):
            child = path.parent / corpus['vector_url']
            child.write_bytes(np.array([1, -0.0 if index else 0.0, 0], dtype='<f2').tobytes())
            corpus['vector_sha256'] = pack.file_sha256(child)
        pack.write_json(path, original)
        with self.assertRaisesRegex(ValueError, 'conflicting vectors'):
            self.build('conflict', rows, '--reuse-from', str(path))

    def test_url_catalog_and_reuse_published_send_user_agent(self):
        previous, path, _, _ = self.build('previous', self.old)
        requests = []
        def download(request, **kwargs):
            self.assertEqual(request.get_header('User-agent'), 'jbotci-embedding-pack-builder/1')
            requests.append(request.full_url)
            relative = request.full_url.removeprefix('https://assets.jbotci.app/embeddings/web/v1/')
            return io.BytesIO((previous / relative).read_bytes())
        for name, flags in [('url', ['--reuse-catalog', 'https://assets.jbotci.app/embeddings/web/v1/catalog.json']),
                            ('published', ['--reuse-published'])]:
            with mock.patch.object(pack, 'urlopen', side_effect=download):
                _, _, embed, _ = self.build(name, self.old, *flags)
            self.assertEqual([call.args[0] for call in embed.call_args_list], [[], []])
        self.assertEqual(len(requests), 12)

    def test_explicit_new_model_refuses_existing_entry(self):
        previous, _, _, _ = self.build('previous', self.old)
        catalog = previous / 'catalog.json'
        with self.assertRaisesRegex(ValueError, 'already has'):
            self.build('existing-model', self.old, '--reuse-catalog', str(catalog), '--new-model', pack.MODEL_KEY)
        pack.write_json(catalog, {'schema_version': 1, 'models': []})
        with self.assertRaisesRegex(ValueError, 'exactly one'):
            self.build('missing-model', self.old, '--reuse-catalog', str(catalog))
        _, _, embed, _ = self.build('new-model', self.old, '--reuse-catalog', str(catalog), '--new-model', pack.MODEL_KEY)
        self.assertEqual(sum(len(call.args[0]) for call in embed.call_args_list), 5)

    def test_external_model_files_hash_raw_bytes_without_loading_weights(self):
        external = self.root / 'weights.bin'
        external.write_bytes(b'raw external weights')
        tensor = types.SimpleNamespace(
            DESCRIPTOR=types.SimpleNamespace(full_name='onnx.TensorProto'), ListFields=lambda: [],
            external_data=[types.SimpleNamespace(key='location', value='weights.bin')])
        field = types.SimpleNamespace(type=11, TYPE_MESSAGE=11, is_repeated=True)
        graph = types.SimpleNamespace(DESCRIPTOR=types.SimpleNamespace(full_name='onnx.ModelProto'), ListFields=lambda: [(field, [tensor])])
        loader = mock.Mock(return_value=graph)
        onnx = types.SimpleNamespace(load=loader, external_data_helper=types.SimpleNamespace(uses_external_data=lambda value: True))
        with mock.patch.dict(sys.modules, {'onnx': onnx}):
            before = pack.external_data_hashes(self.model)
            external.write_bytes(b'changed raw weights')
            after = pack.external_data_hashes(self.model)
        self.assertNotEqual(before, after)
        self.assertEqual(after, {'weights.bin': pack.sha256(b'changed raw weights')})
        loader.assert_called_with(self.model, load_external_data=False)

    def test_validator_accepts_new_metadata_and_refuses_duplicate_corpora(self):
        spec = importlib.util.spec_from_file_location('validate_vectors', SCRIPT.with_name('validate-vector-pack.py'))
        validator = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(validator)
        # SCRIPT is tools/embedding-pack/f2llm/build-vector-pack.py.
        fixture = SCRIPT.parents[3] / 'crates/jbotci-f2llm-runtime/testdata/manifests/v1/incremental-80m.json'
        manifest = pack.read_json(fixture)
        args = types.SimpleNamespace(model_key=manifest['model_key'], max_sequence_length=manifest['max_sequence_length'],
                                     dimensions=manifest['dimensions'], include_wasm_runtime=False)
        corpus = {'inputFormatVersion': manifest['input_format_version'], 'inputHash': manifest['input_hash']}
        validator.validate_manifest(manifest, corpus, args)
        manifest['corpora'].append(manifest['corpora'][0])
        with self.assertRaisesRegex(AssertionError, 'corpora mismatch'):
            validator.validate_manifest(manifest, corpus, args)

    @unittest.skipIf(real_onnx is None, 'requires onnx from the builder requirements')
    def test_real_onnx_external_data_hashes_graph_and_raw_files(self):
        external = self.root / 'real.weights'
        external.write_bytes(np.array([1.0], dtype='<f4').tobytes())
        tensor = real_onnx.TensorProto(name='weight', data_type=real_onnx.TensorProto.FLOAT, dims=[1],
                                      data_location=real_onnx.TensorProto.EXTERNAL)
        tensor.external_data.add(key='location', value=external.name)
        model = real_onnx.helper.make_model(real_onnx.helper.make_graph([], 'tiny-hash-test', [], [], [tensor]))
        graph = self.root / 'real.onnx'
        raw_graph = model.SerializeToString()
        graph.write_bytes(raw_graph)
        with mock.patch.dict(sys.modules, {'onnx': real_onnx}), mock.patch.object(real_onnx, 'load', wraps=real_onnx.load) as load:
            before = pack.external_data_hashes(graph)
            external.write_bytes(np.array([2.0], dtype='<f4').tobytes())
            after = pack.external_data_hashes(graph)
        self.assertNotEqual(before, after)
        self.assertEqual(pack.file_sha256(graph), pack.sha256(raw_graph))
        self.assertEqual(after, {external.name: pack.file_sha256(external)})
        for call in load.call_args_list:
            self.assertFalse(call.kwargs['load_external_data'])


if __name__ == '__main__':
    unittest.main()
