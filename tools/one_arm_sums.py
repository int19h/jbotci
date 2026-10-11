"""Compare the eight approved wrapper removals. Reject all other changes."""
import json
import re
import sys
sys.setrecursionlimit(100000)

ARMS = {
    'RegularText': 'TextSyntax',
    'BridiTailBoContinuation': 'BridiTailBoJointSyntax',
    'BridiTailBoContinuationWithoutTailTerms': 'BridiTailBoJointWithoutTailTermsSyntax',
    'StagBoundTermContinuation': 'BoundTermContinuationSyntax',
    'BoundNormalTermContinuation': 'NormalTermBoContinuationSyntax',
    'BoundSumtiTail': 'SumtiBoundTailSyntax',
    'StandardForethoughtSelbriConnection': 'ForethoughtSelbriConnectionSyntax',
    'TanruUnitAtom': 'TanruUnitAtomSyntax',
}
RULE_NAMES = {
    'regular_text': 'text',
    'bridi_tail_bo_continuation': 'bridi_tail_bo_joint',
    'bridi_tail_bo_continuation_without_tail_terms': 'bridi_tail_bo_joint_without_tail_terms',
    'stag_bound_term_continuation': 'bound_term_continuation',
    'bound_normal_term_continuation': 'normal_term_bo_continuation',
    'bound_sumti_tail': 'sumti_bound_tail',
    'standard_forethought_selbri_connection': 'forethought_selbri_connection',
    # This shared arm keeps its rule name. The scalar wrapper disappears.
    'tanru_unit_atom': 'tanru_unit_atom',
}
RENAMES = {arm+'Syntax': target for arm, target in ARMS.items()}
TOKEN = re.compile(r'@\[[0-9]+‥[0-9]+\)|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|[A-Za-z_][A-Za-z_0-9]*|[0-9]+(?:\.[0-9]+)?|[^\s]')
PAIRS = {'(': ')', '[': ']', '{': '}'}

def debug_tokens(text, old=False):
    tokens = TOKEN.findall(text)
    position = 0

    def sequence(close=None):
        nonlocal position
        result = []
        while position < len(tokens):
            token = tokens[position]
            position += 1
            if token == close:
                return result
            if token in PAIRS:
                result.append((token, sequence(PAIRS[token])))
            else:
                assert token not in PAIRS.values(), ('unpaired delimiter', token)
                result.append(token)
        assert close is None, ('missing delimiter', close)
        return result

    def without_trailing_comma(items):
        return items[:-1] if items and items[-1] == ',' else items

    def call(items, names):
        items = without_trailing_comma(items)
        if len(items) == 2 and isinstance(items[0], str) and (items[0] in names) and isinstance(items[1], tuple) and (items[1][0] == '('):
            return (items[0], without_trailing_comma(items[1][1]))
        return None

    def transform(items, owner=None):
        result = []
        i = 0
        while i < len(items):
            item = items[i]
            if old and item == 'Valid' and (i + 1 < len(items)) and isinstance(items[i + 1], tuple) and (items[i + 1][0] == '('):
                nested = call(items[i + 1][1], ARMS)
                if nested:
                    result.extend(transform(nested[1]))
                    i += 2
                    continue
            if old and isinstance(item, str) and (item in ARMS) and (i + 1 < len(items)) and isinstance(items[i + 1], tuple) and (items[i + 1][0] == '('):
                inside = without_trailing_comma(items[i + 1][1])
                valid = call(inside, {'Valid'})
                if valid:
                    inside = valid[1]
                result.extend(transform(inside))
                i += 2
                continue
            if isinstance(item, tuple):
                item = (item[0], transform(item[1], items[i - 1] if i and isinstance(items[i - 1], str) else None))
            elif old and owner == 'MissingRequiredField' and (i >= 2) and (items[i - 2:i] == ['expected', ':']):
                value = rust_string(item)
                if value in RULE_NAMES and RULE_NAMES[value] != value:
                    item = json.dumps(RULE_NAMES[value])
            elif old:
                item = RENAMES.get(item, item)
            result.append(item)
            i += 1
        return without_trailing_comma(result)
    return transform(sequence())

def json_value(text, old=False):

    def walk(value):
        if isinstance(value, list):
            return [walk(item) for item in value]
        if not isinstance(value, dict):
            return value
        if old and len(value) == 1 and (next(iter(value)) in ARMS):
            return walk(next(iter(value.values())))
        return {key: walk(item) for key, item in value.items()}
    return walk(json.loads(text))

def rust_string(token):
    assert token.startswith('"') and token.endswith('"')
    result = []
    i = 1
    while i < len(token) - 1:
        char = token[i]
        i += 1
        if char != '\\':
            result.append(char)
            continue
        char = token[i]
        i += 1
        escapes = {'n': '\n', 'r': '\r', 't': '\t', '0': '\x00', '\\': '\\', '"': '"'}
        if char in escapes:
            result.append(escapes[char])
        elif char == 'u' and token[i] == '{':
            end = token.index('}', i + 1)
            result.append(chr(int(token[i + 1:end], 16)))
            i = end + 1
        elif char == 'x':
            result.append(chr(int(token[i:i + 2], 16)))
            i += 2
        else:
            raise ValueError(('unknown Rust escape', char))
    return ''.join(result)

def tree_tokens(text, old=False):
    if text.startswith('Ok(') and text.endswith(')'):
        text = rust_string(text[3:-1])
    parsed = debug_tokens(text)
    names = {arm: target.removesuffix('Syntax') for arm, target in ARMS.items()}

    def walk(items):
        result = []
        for index, item in enumerate(items):
            if isinstance(item, tuple):
                item = (item[0], walk(item[1]))
            elif old and item in names:
                next_index = index + 1
                if next_index < len(items) and isinstance(items[next_index], str) and items[next_index].startswith('@['):
                    next_index += 1
                if next_index < len(items) and isinstance(items[next_index], tuple) and (items[next_index][0] == '{'):
                    item = names[item]
            result.append(item)
        return result
    return walk(parsed)



def rename_tree_headings(text):
    names = {arm: target.removesuffix("Syntax") for arm, target in ARMS.items()}
    tokens = list(TOKEN.finditer(text))
    replacements = []
    for index, token in enumerate(tokens):
        name = token.group()
        if name not in names or names[name] == name:
            continue
        following = index + 1
        if following < len(tokens) and tokens[following].group().startswith("@["):
            following += 1
        if following < len(tokens) and tokens[following].group() == "{":
            replacements.append((token.start(), token.end(), names[name]))
    for start, end, value in reversed(replacements):
        text = text[:start] + value + text[end:]
    return text

def recovered_json_value(text, old=False):
    names = {arm: target.removesuffix('Syntax') for arm, target in ARMS.items()}

    def walk(value):
        if isinstance(value, list):
            return [walk(item) for item in value]
        if isinstance(value, dict):
            return {names.get(key, key) if old else key: walk(item) for key, item in value.items()}
        return value
    return walk(json.loads(text))

def compare(mode, before, after):
    if before == after:
        return 'identical'
    if mode in ('raw', 'recovered_raw') and debug_tokens(before, True) == debug_tokens(after):
        return 'wrapper-only'
    if mode == 'json' and json_value(before, True) == json_value(after):
        return 'wrapper-only'
    if mode == 'recovered_json' and (json_value(before, True) == json_value(after) or recovered_json_value(before, True) == recovered_json_value(after)):
        return 'wrapper-only'
    if mode in ('tree', 'recovered_tree') and tree_tokens(before, True) == tree_tokens(after):
        return 'wrapper-only'
    return 'other'

def self_test():
    for old, new in RULE_NAMES.items():
        before = f'MissingRequiredField {{ expected: "{old}", error_index: 0 }}'
        after = f'MissingRequiredField {{ expected: "{new}", error_index: 0 }}'
        assert compare('raw', before, after) in ('identical', 'wrapper-only')
        assert compare('raw', before, after.replace('error_index: 0', 'error_index: 1')) == 'other'
        if old != new:
            assert compare('raw', f'Word {{ text: "{old}" }}', f'Word {{ text: "{new}" }}') == 'other'
            assert compare('raw', before, after.replace(new, new + 'x')) == 'other'
    assert compare('raw', 'MissingRequiredField { expected: "bad\\u{61}" }', 'MissingRequiredField { expected: "bada" }') == 'other'
    assert compare('json', '{"RegularText":{"paragraphs":null}}', '{"paragraphs":null}') == 'wrapper-only'
    assert compare('json', '{"RegularText":{"paragraphs":"old"}}', '{"paragraphs":"new"}') == 'other'
    assert compare('raw', 'RegularText(RegularTextSyntax { paragraphs: None })', 'TextSyntax { paragraphs: None }') == 'wrapper-only'
    assert compare('raw', 'RegularText(RegularTextSyntax { paragraphs: None })', 'TextSyntax { paragraphs: Some(0) }') == 'other'
    assert compare('tree', 'RegularText @[0‥10) { child: "x" }', 'Text @[0‥10) { child: "x" }') == 'wrapper-only'
    assert compare('tree', 'RegularText @[0‥10) { child: "x" }', 'Text @[0‥11) { child: "x" }') == 'other'
    print('All eight mappings pass. Unknown names and unrelated changes fail.')
if __name__ == '__main__':
    self_test()
