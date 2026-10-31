from __future__ import annotations

import contextlib
import copy
import importlib.util
import io
import json
import os
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
sys.modules[SPEC.name] = pack
SPEC.loader.exec_module(pack)
external_data_hashes = pack.external_data_hashes


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
        hashes = mock.patch.object(pack, 'external_data_hashes', return_value={})
        hashes.start()
        self.addCleanup(hashes.stop)
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

    def contents(self, root):
        return {str(p.relative_to(root)): p.read_bytes() for p in root.rglob('*') if p.is_file()}

    def interrupt(self, name, rows, *flags):
        original = pack.embed_texts
        calls = 0
        def failing(*args, **kwargs):
            nonlocal calls
            calls += 1
            if calls == 3:
                raise RuntimeError('injected backend failure')
            return original(*args, **kwargs)
        with mock.patch.object(pack, 'embed_texts', side_effect=failing):
            with self.assertRaisesRegex(RuntimeError, 'injected backend failure'):
                self.build(name, rows, '--chunk-rows', '1', *flags)
        partial = self.root / f'{name}.partial'
        checkpoint = pack.read_json(partial / 'checkpoint.json')
        self.assertEqual(len(checkpoint['chunks']), 2)
        self.assertFalse((self.root / name).exists())
        return partial

    def test_resume_only_embeds_unfinished_rows_and_matches_full_bytes(self):
        full, _, _, _ = self.build('full', self.old, '--chunk-rows', '1')
        partial = self.interrupt('retry', self.old)
        retry, _, embed, output = self.build('retry', self.old, '--chunk-rows', '1')
        self.assertEqual([c.args[0] for c in embed.call_args_list],
                         [['remove'], ['CLL keep'], ['another CLL windowed document']])
        self.assertIn('resumed 2 chunks (2 rows)', output)
        self.assertEqual(self.contents(full), self.contents(retry))
        self.assertFalse(partial.exists())

    def test_windows_checkpoint_replacement_and_resume(self):
        original = pack.durable_replace
        seen = []

        def move(source, destination, flags):
            self.assertEqual(flags, 0x1 | 0x8)
            seen.append((Path(destination).name, Path(destination).exists()))
            os.replace(source, destination)
            return 1

        api = mock.Mock(side_effect=move)
        kernel = types.SimpleNamespace(MoveFileExW=api)
        with mock.patch.object(pack.ctypes, 'WinDLL', return_value=kernel, create=True), \
                mock.patch.object(pack, 'durable_replace',
                                  side_effect=lambda a, b: original(a, b, platform='nt')):
            full, _, _, _ = self.build('full', self.old, '--chunk-rows', '1')
            partial = self.interrupt('retry', self.old)
            checkpoint = pack.read_json(partial / 'checkpoint.json')
            self.assertEqual(len(checkpoint['chunks']), 2)
            retry, _, embed, output = self.build('retry', self.old, '--chunk-rows', '1')
        self.assertIn(('checkpoint.json', False), seen)
        self.assertIn(('checkpoint.json', True), seen)
        self.assertEqual([c.args[0] for c in embed.call_args_list],
                         [['remove'], ['CLL keep'], ['another CLL windowed document']])
        self.assertIn('resumed 2 chunks (2 rows)', output)
        self.assertEqual(self.contents(full), self.contents(retry))
        self.assertFalse(partial.exists())

    def test_atomic_write_propagates_sync_and_windows_move_errors(self):
        target = self.root / 'atomic.json'
        target.write_bytes(b'old')
        # A failed file sync must not replace the destination.
        with mock.patch.object(pack.os, 'fsync', side_effect=OSError('file sync failed')):
            with self.assertRaisesRegex(OSError, 'file sync failed'):
                pack.atomic_write(target, b'new')
        self.assertEqual(target.read_bytes(), b'old')
        # A directory sync error must also reach the caller.
        with mock.patch.object(pack.os, 'open', return_value=123), \
                mock.patch.object(pack.os, 'close'), \
                mock.patch.object(pack.os, 'fsync', side_effect=OSError('directory sync failed')):
            with self.assertRaisesRegex(OSError, 'directory sync failed'):
                pack.durable_replace(target.with_name('atomic.json.tmp'), target, platform='posix')
        kernel = types.SimpleNamespace(MoveFileExW=mock.Mock(return_value=0))
        with mock.patch.object(pack.ctypes, 'WinDLL', return_value=kernel, create=True), \
                mock.patch.object(pack.ctypes, 'get_last_error', return_value=5, create=True), \
                mock.patch.object(pack.ctypes, 'WinError', return_value=OSError('write-through failed'), create=True):
            with self.assertRaisesRegex(OSError, 'write-through failed'):
                pack.durable_replace(target, self.root / 'other.json', platform='nt')

    def test_backend_failure_leaves_only_completed_chunks(self):
        rows = {'dictionary': [document(i, str(i)) for i in range(5)], 'cll': []}
        full, _, _, _ = self.build('full', rows, '--chunk-rows', '1')
        original = Session.run
        calls = 0
        def fail_third(session, *args):
            nonlocal calls
            calls += 1
            if calls == 3:
                raise RuntimeError('stub backend stopped')
            return original(session, *args)
        with mock.patch.object(Session, 'run', fail_third):
            with self.assertRaisesRegex(RuntimeError, 'stub backend stopped'):
                self.build('retry', rows, '--chunk-rows', '1')
        with mock.patch.object(Session, 'run', autospec=True, side_effect=original) as backend:
            retry, _, _, _ = self.build('retry', rows, '--chunk-rows', '1')
        self.assertEqual(backend.call_count, 3)
        self.assertEqual(self.contents(full), self.contents(retry))

    def test_checkpoint_corpus_identity_and_layout_changes_discard_all(self):
        for change in ('corpus', 'model', 'tokenizer', 'layout', 'fresh'):
            with self.subTest(change=change):
                name = 'retry-' + change
                self.interrupt(name, self.old)
                rows = self.new if change == 'corpus' else self.old
                flags = ['--chunk-rows', '2' if change == 'layout' else '1']
                if change == 'model':
                    self.model.write_bytes(b'changed-model')
                if change == 'fresh':
                    flags += ['--discard-checkpoint']
                with mock.patch.object(Tokenizer.backend_tokenizer, 'to_str',
                                       return_value='changed' if change == 'tokenizer' else 'test-tokenizer-v1'):
                    full, _, _, _ = self.build('full-' + change, rows, *flags)
                    retry, _, embed, output = self.build(name, rows, *flags)
                self.assertIn('Discarding checkpoint', output)
                self.assertEqual(sum(len(c.args[0]) for c in embed.call_args_list), 5)
                self.assertEqual(self.contents(full), self.contents(retry))
                self.model.write_bytes(b'test-model')

    def test_bad_chunk_reembeds_only_its_rows(self):
        full, _, _, _ = self.build('full', self.old, '--chunk-rows', '1')
        for kind in ('missing', 'corrupt'):
            with self.subTest(kind=kind):
                name = 'retry-' + kind
                partial = self.interrupt(name, self.old)
                chunk = partial / 'vlacku-en-0.f16'
                if kind == 'missing':
                    chunk.unlink()
                else:
                    chunk.write_bytes(b'xxxxxx')
                retry, _, embed, output = self.build(name, self.old, '--chunk-rows', '1')
                self.assertEqual(sum(len(c.args[0]) for c in embed.call_args_list), 4)
                self.assertNotIn(['change'], [c.args[0] for c in embed.call_args_list])
                self.assertIn('Discarding checkpoint chunk vlacku-en-0.f16', output)
                self.assertEqual(self.contents(full), self.contents(retry))

    def test_resume_with_reuse_matches_uninterrupted_pack(self):
        _, previous, _, _ = self.build('previous', self.old)
        flags = ['--reuse-from', str(previous)]
        full, _, _, _ = self.build('full', self.new, '--chunk-rows', '1', *flags)
        self.interrupt('retry', self.new, *flags)
        retry, _, embed, output = self.build('retry', self.new, '--chunk-rows', '1', *flags)
        self.assertEqual([c.args[0] for c in embed.call_args_list], [['added'], [], []])
        self.assertIn('cukta-cll: reused 2 rows, embedded 0 rows', output)
        self.assertIn('resumed 2 chunks (2 rows)', output)
        self.assertEqual(self.contents(full), self.contents(retry))

    def test_orphan_chunk_after_checkpoint_write_failure_is_recomputed(self):
        original = pack.atomic_write
        calls = 0
        def fail_checkpoint(path, content):
            nonlocal calls
            if path.name == 'checkpoint.json':
                calls += 1
                if calls == 2:
                    raise RuntimeError('checkpoint write failed')
            return original(path, content)
        with mock.patch.object(pack, 'atomic_write', side_effect=fail_checkpoint):
            with self.assertRaisesRegex(RuntimeError, 'checkpoint write failed'):
                self.build('retry', self.old, '--chunk-rows', '1')
        self.assertTrue((self.root / 'retry.partial/vlacku-en-0.f16').exists())
        full, _, _, _ = self.build('full', self.old, '--chunk-rows', '1')
        retry, _, embed, _ = self.build('retry', self.old, '--chunk-rows', '1')
        self.assertEqual(sum(len(c.args[0]) for c in embed.call_args_list), 5)
        self.assertEqual(self.contents(full), self.contents(retry))

    def test_completed_aggregate_part_reuses_checkpoint_without_backend_calls(self):
        full, _, _, _ = self.build('full', self.old)
        self.build('part', self.old, '--keep-work')
        self.assertTrue((self.root / 'part.partial').exists())
        retry, _, embed, output = self.build('part', self.old, '--overwrite')
        self.assertEqual(embed.call_count, 0)
        self.assertIn('resumed 1 chunks (3 rows)', output)
        self.assertEqual(self.contents(full), self.contents(retry))
        self.assertFalse((self.root / 'part.partial').exists())

    def test_explicit_work_directory_and_path_separation(self):
        work = self.root / 'saved-chunks'
        with mock.patch.object(pack, 'promote', side_effect=RuntimeError('promotion failed')):
            with self.assertRaisesRegex(RuntimeError, 'promotion failed'):
                self.build('retry', self.old, '--work-dir', str(work))
        self.assertTrue(work.is_dir())
        _, _, embed, _ = self.build('retry', self.old, '--work-dir', str(work))
        self.assertEqual(embed.call_count, 0)
        self.assertFalse(work.exists())
        for path in (self.root / 'bad', self.root, self.root / 'bad.staging'):
            with self.assertRaisesRegex(ValueError, 'non-overlapping'):
                self.build('bad', self.old, '--work-dir', str(path))

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

    def test_validator_accepts_new_metadata_and_refuses_duplicate_corpora(self):
        spec = importlib.util.spec_from_file_location('validate_vectors', SCRIPT.with_name('validate-vector-pack.py'))
        validator = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(validator)
        # SCRIPT is tools/embedding-pack/f2llm/build-vector-pack.py.
        fixture = SCRIPT.parents[3] / 'crates/jbotci-f2llm-runtime/testdata/manifests/v1/incremental-80m.json'
        manifest = pack.read_json(fixture)
        manifest['artifact_version'] = validator.ARTIFACT_VERSION
        manifest['vector_space_key'] = validator.VECTOR_SPACE_KEY
        for runtime in manifest['compatible_query_runtimes']:
            runtime['version'] = validator.RUNTIME_VERSION
        args = types.SimpleNamespace(model_key=manifest['model_key'], max_sequence_length=manifest['max_sequence_length'],
                                     dimensions=manifest['dimensions'], include_wasm_runtime=False, vector_space_key=validator.VECTOR_SPACE_KEY)
        corpus = {'inputFormatVersion': manifest['input_format_version'], 'inputHash': manifest['input_hash']}
        validator.validate_manifest(manifest, corpus, args)
        manifest['corpora'].append(manifest['corpora'][0])
        with self.assertRaisesRegex(AssertionError, 'corpora mismatch'):
            validator.validate_manifest(manifest, corpus, args)

    def test_validator_rejects_invalid_unsampled_rows_before_inference(self):
        spec = importlib.util.spec_from_file_location('validate_invalid_vectors', SCRIPT.with_name('validate-vector-pack.py'))
        validator = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(validator)
        for invalid in [[0, 0, 0], [float('nan'), 1, 0], [float('inf'), 1, 0]]:
            with self.subTest(invalid=invalid):
                output, manifest_path, _, _ = self.build('invalid', self.old, '--overwrite')
                manifest = pack.read_json(manifest_path)
                corpus = manifest['corpora'][0]
                path = manifest_path.parent / corpus['vector_url']
                rows = np.frombuffer(path.read_bytes(), dtype='<f2').copy().reshape(-1, 3)
                rows[2] = invalid
                # Row two is outside a one-row numerical sample.
                self.assertNotIn(2, validator.sample_rows(len(rows), 1))
                path.write_bytes(rows.tobytes())
                corpus['vector_sha256'] = pack.sha256(path.read_bytes())
                pack.write_json(manifest_path, manifest)
                corpus_path = self.root / 'invalid-input.json'
                args = ['validate-vector-pack.py', '--pack', str(output), '--corpus', str(corpus_path),
                        '--q4-onnx', str(self.model), '--dimensions', '3', '--max-sequence-length', '8', '--sample-rows', '1']
                with mock.patch.object(sys, 'argv', args), \
                        mock.patch.object(validator.AutoTokenizer, 'from_pretrained', side_effect=AssertionError('inference must not start')):
                    with self.assertRaisesRegex(AssertionError, 'invalid embedding row 2'):
                        validator.main()
        with self.assertRaisesRegex(AssertionError, 'invalid embedding row'):
            validator.token_weighted_mean_normalized([np.array([1., 0.]), np.array([-1., 0.])], [1, 1], 2)

    def test_every_window_ends_with_eos_and_the_short_tail_has_its_token_weight(self):
        modules = [pack]
        for filename in ['embedding-pack/f2llm/validate-vector-pack.py', 'f2llm-oracles/generate-f2llm-goldens.py']:
            spec = importlib.util.spec_from_file_location(filename, SCRIPT.parents[2] / filename)
            module = importlib.util.module_from_spec(spec)
            sys.modules[filename] = module
            spec.loader.exec_module(module)
            modules.append(module)
        for module in modules:
            for count in [0, 510, 511, 512, 1021, 1022, 1023]:
                windows = module.token_windows('x' * count, Tokenizer(), 512)
                self.assertEqual(len(windows), max(1, (count + 510) // 511))
                self.assertTrue(all(len(w) <= 512 and w[-1] == 1 for w in windows))
                self.assertEqual(sum(len(w) - 1 for w in windows), count)
                self.assertTrue(count == 0 or all(len(w) > 1 for w in windows))
            pooled = module.token_weighted_mean_normalized([np.array([1., 0.]), np.array([0., 1.])], [512, 2], 2)
            self.assertAlmostEqual(float(pooled[1] / pooled[0]), 2 / 512, places=7)

    def test_old_vector_space_cannot_supply_rows_or_resume(self):
        previous, _, _, _ = self.build('old-space', self.old, '--vector-space-key', 'jbotci-browser-f2llm-q4-f16-windowed-512-v2')
        with self.assertRaisesRegex(ValueError, 'vector_space_key'):
            self.build('new-space', self.old, '--reuse-from', str(next(previous.rglob('manifest.json'))))
        self.interrupt('old-checkpoint', self.old, '--vector-space-key', 'jbotci-browser-f2llm-q4-f16-windowed-512-v2')
        _, _, embed, output = self.build('old-checkpoint', self.old, '--chunk-rows', '1')
        self.assertEqual(sum(len(call.args[0]) for call in embed.call_args_list), 5)
        self.assertIn('Discarding checkpoint', output)

    @unittest.skipIf(real_onnx is None, 'requires onnx from the builder requirements')
    def test_real_onnx_external_data_hashes_graph_and_raw_files(self):
        external = self.root / 'real.weights'
        external.write_bytes(np.array([1.0], dtype='<f4').tobytes())
        tensor = real_onnx.TensorProto(name='weight', data_type=real_onnx.TensorProto.FLOAT, dims=[1],
                                      data_location=real_onnx.TensorProto.EXTERNAL)
        tensor.external_data.add(key='location', value=external.name)
        inline = real_onnx.helper.make_tensor('inline', real_onnx.TensorProto.FLOAT, [8192], b'x' * 32768, raw=True)
        node = real_onnx.helper.make_node('Constant', [], ['x'], value=tensor)
        model = real_onnx.helper.make_model(real_onnx.helper.make_graph([node], 'tiny-hash-test', [], [], [tensor, inline]))
        model.functions.append(real_onnx.helper.make_function('test', 'constant', [], ['x'], [node],
                                                            [real_onnx.helper.make_opsetid('', 13)]))
        graph = self.root / 'real.onnx'
        raw_graph = model.SerializeToString()
        graph.write_bytes(raw_graph)
        with mock.patch.dict(sys.modules, {'onnx': real_onnx}), mock.patch.object(real_onnx, 'load', side_effect=AssertionError('must not load the model')) as load:
            before = external_data_hashes(graph)
            external.write_bytes(np.array([2.0], dtype='<f4').tobytes())
            after = external_data_hashes(graph)
        self.assertNotEqual(before, after)
        self.assertEqual(pack.file_sha256(graph), pack.sha256(raw_graph))
        self.assertEqual(after, {external.name: pack.file_sha256(external)})
        load.assert_not_called()
        class RecordingFile(io.BytesIO):
            bytes_read = 0
            def read(self, size=-1):
                value = super().read(size)
                self.bytes_read += len(value)
                return value
        source = RecordingFile(raw_graph)
        with mock.patch.dict(sys.modules, {'onnx': real_onnx}):
            pack.scan_external_data(source, real_onnx.ModelProto.DESCRIPTOR, len(raw_graph), set())
        self.assertLess(source.bytes_read, 1024, 'the scanner must seek past inline tensor payloads')

    def test_streaming_metadata_refuses_truncated_protobuf(self):
        for data in [b'\x80', b'\x80' * 10, b'\x00', b'\x0a\x10short', b'\x0c', b'\x0b\x14']:
            descriptor = types.SimpleNamespace(full_name='unknown', fields_by_number={})
            with self.subTest(data=data), self.assertRaises(ValueError):
                pack.scan_external_data(io.BytesIO(data), descriptor, len(data), set())

    @unittest.skipIf(real_onnx is None, 'requires onnx from the builder requirements')
    def test_streamed_metadata_obeys_protobuf_message_merging(self):
        def varint(value):
            result = bytearray()
            while value > 127:
                result.append((value & 127) | 128)
                value >>= 7
            result.append(value)
            return bytes(result)
        def message(field, payload):
            return varint((field << 3) | 2) + varint(len(payload)) + payload
        first = real_onnx.TensorProto(data_location=real_onnx.TensorProto.EXTERNAL)
        first.external_data.add(key='location', value='old.weights')
        second = real_onnx.TensorProto()
        second.external_data.add(key='location', value='current.weights')
        # AttributeProto.t is singular: protobuf merges its two occurrences.
        tensor_field = real_onnx.AttributeProto.DESCRIPTOR.fields_by_name['t'].number
        raw = message(tensor_field, first.SerializeToString()) + message(tensor_field, second.SerializeToString())
        locations = set()
        with mock.patch.dict(sys.modules, {'onnx': real_onnx}):
            pack.scan_external_data(io.BytesIO(raw), real_onnx.AttributeProto.DESCRIPTOR, len(raw), locations)
        self.assertEqual(locations, {'current.weights'})


if __name__ == '__main__':
    unittest.main()
