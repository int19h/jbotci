"""Static, fail-closed audit of type-bearing declarations in manual stubs.

The audit deliberately interprets only the finite subset of Python needed by
stub declarations.  Bindings are represented by finite provenance atoms, and
each syntactic definition has one monotonically growing capture.  Repeated
control-flow analysis therefore reaches a fixed point without losing a
possible origin or manufacturing an unbounded chain of environments.
"""

from __future__ import annotations

import ast
from dataclasses import dataclass, field


@dataclass(frozen=True)
class _Binding:
    kind: str
    definition: int | None = None


_BindingSet = frozenset[_Binding]
_Bindings = dict[str, _BindingSet]
_ForwardStringKey = tuple[
    str,
    int,
    frozenset[tuple[str, _BindingSet]],
]

_OTHER = _Binding("other")

_TYPING_KINDS: dict[str, str] = {
    "Any": "any",
    "Literal": "literal",
    "Annotated": "annotated",
    "TypeAlias": "type-alias",
    "TypeAliasType": "type-alias-type",
    "TypeVar": "type-var",
    "ParamSpec": "param-spec",
    "TypeVarTuple": "type-var-tuple",
    "NewType": "new-type",
    "ForwardRef": "forward-ref",
    "NamedTuple": "named-tuple",
    "TypedDict": "typed-dict",
    "cast": "cast",
    "assert_type": "assert-type",
}

_BUILTIN_KINDS: dict[str, str] = {
    "dict": "builtin-dict",
    "list": "builtin-list",
    "tuple": "builtin-tuple",
    "object": "catch-all",
}


@dataclass
class _Scope:
    parent: _Scope | None
    parent_bindings: _Bindings
    final_bindings: _Bindings = field(default_factory=dict)


@dataclass
class _Environment:
    scope: _Scope
    bindings: _Bindings


@dataclass
class _Definition:
    expression: ast.expr
    scope: _Scope
    captures: _Bindings = field(default_factory=dict)


@dataclass
class _TypeRoot:
    label: str
    expression: ast.expr
    context: str
    scope: _Scope
    type_candidates: tuple[ast.expr, ...] = ()
    captures: _Bindings = field(default_factory=dict)


@dataclass(frozen=True)
class _LocatedExpression:
    expression: ast.expr
    captures: _Bindings
    scope: _Scope


@dataclass(frozen=True)
class _CallShape:
    positional: tuple[_LocatedExpression, ...] = ()
    keywords: tuple[tuple[str, _LocatedExpression], ...] = ()


@dataclass
class _Flow:
    normal: _Environment | None = None
    breaks: _Environment | None = None
    continues: _Environment | None = None
    exceptions: _Environment | None = None


@dataclass(frozen=True)
class _Problem:
    description: str
    line: int


def _copy_bindings(bindings: _Bindings) -> _Bindings:
    return dict(bindings)


def _merge_bindings(destination: _Bindings, source: _Bindings) -> bool:
    changed = False
    for name, incoming in source.items():
        merged = destination.get(name, frozenset()) | incoming
        if merged != destination.get(name):
            destination[name] = merged
            changed = True
    return changed


def _binding_maps_equal(left: _Bindings, right: _Bindings) -> bool:
    return left == right


class _Analyzer:
    def __init__(self) -> None:
        self._definitions: dict[tuple[int, int], _Definition] = {}
        self._definitions_by_identity: dict[int, _Definition] = {}
        self._roots: dict[tuple[int, int, str, str], _TypeRoot] = {}
        self._class_scopes: dict[int, _Scope] = {}
        self._issues: set[str] = set()
        self._version = 0

    def analyze(self, tree: ast.Module) -> tuple[str, ...]:
        scope = _Scope(parent=None, parent_bindings={})
        environment = _Environment(scope=scope, bindings={})
        flow = self._statements(tree.body, environment, class_scope=False)
        if flow.normal is not None:
            self._update_scope_final(scope, flow.normal.bindings)

        for root in self._roots.values():
            context = root.context
            if context == "typed-dict-extra-items":
                if not any(
                    "typed-dict"
                    in self._possible_kinds(
                        candidate,
                        root.captures,
                        root.scope,
                        set(),
                    )
                    for candidate in root.type_candidates
                ):
                    continue
                context = "type"
            if context == "class-keyword-unpack":
                typed_dict = any(
                    "typed-dict"
                    in self._possible_kinds(
                        candidate,
                        root.captures,
                        root.scope,
                        set(),
                    )
                    for candidate in root.type_candidates
                )
                problems = (
                    self._scan_class_keyword_unpack(
                        root.expression,
                        root.captures,
                        root.scope,
                        visiting=set(),
                        strings=set(),
                    )
                    if typed_dict
                    else set()
                )
            else:
                problems = self._scan(
                    root.expression,
                    root.captures,
                    root.scope,
                    context,
                    set(),
                    set(),
                )
            for problem in problems:
                self._issues.add(
                    f"line {problem.line}: {root.label} {problem.description}"
                )
        return tuple(sorted(self._issues))

    def _scope_parent_visible(self, scope: _Scope) -> _Bindings:
        visible = _copy_bindings(scope.parent_bindings)
        parent = scope.parent
        while parent is not None:
            for name, bindings in parent.final_bindings.items():
                visible.setdefault(name, bindings)
            parent = parent.parent
        return visible

    def _visible(self, environment: _Environment) -> _Bindings:
        visible = self._scope_parent_visible(environment.scope)
        visible.update(environment.bindings)
        return visible

    def _lookup(
        self, name: str, captures: _Bindings, scope: _Scope
    ) -> _BindingSet:
        if name in captures:
            return captures[name]
        current: _Scope | None = scope
        while current is not None:
            if name in current.final_bindings:
                return current.final_bindings[name]
            if name in current.parent_bindings:
                return current.parent_bindings[name]
            current = current.parent
        builtin = _BUILTIN_KINDS.get(name)
        return frozenset({_Binding(builtin)}) if builtin is not None else frozenset()

    def _update_scope_final(self, scope: _Scope, bindings: _Bindings) -> None:
        if _merge_bindings(scope.final_bindings, bindings):
            self._version += 1

    def _capture(self, destination: _Bindings, environment: _Environment) -> None:
        if _merge_bindings(destination, self._visible(environment)):
            self._version += 1

    def _add_root(
        self,
        label: str,
        expression: ast.expr,
        environment: _Environment,
        context: str,
        *,
        type_candidates: tuple[ast.expr, ...] = (),
    ) -> None:
        key = (
            id(expression),
            id(environment.scope),
            label,
            context,
        )
        root = self._roots.get(key)
        if root is None:
            root = _TypeRoot(
                label,
                expression,
                context,
                environment.scope,
                type_candidates,
            )
            self._roots[key] = root
            self._version += 1
        self._capture(root.captures, environment)

    def _definition_binding(
        self, expression: ast.expr, environment: _Environment
    ) -> _Binding:
        key = (id(expression), id(environment.scope))
        definition = self._definitions.get(key)
        if definition is None:
            definition = _Definition(expression, environment.scope)
            self._definitions[key] = definition
            self._definitions_by_identity[id(definition)] = definition
            self._version += 1
        self._capture(definition.captures, environment)
        return _Binding("expression", id(definition))

    def _definition_for(self, binding: _Binding) -> _Definition | None:
        if binding.definition is None:
            return None
        return self._definitions_by_identity.get(binding.definition)

    def _replace(
        self, environment: _Environment, replacements: _Bindings
    ) -> _Environment:
        bindings = _copy_bindings(environment.bindings)
        bindings.update(replacements)
        return _Environment(environment.scope, bindings)

    def _delete(
        self, environment: _Environment, names: set[str]
    ) -> _Environment:
        bindings = _copy_bindings(environment.bindings)
        for name in names:
            bindings.pop(name, None)
        return _Environment(environment.scope, bindings)

    def _join(
        self, *environments: _Environment | None
    ) -> _Environment | None:
        reachable = [environment for environment in environments if environment is not None]
        if not reachable:
            return None
        scope = reachable[0].scope
        if any(environment.scope is not scope for environment in reachable):
            raise AssertionError("cannot join environments from different lexical scopes")
        names = {
            name
            for environment in reachable
            for name in self._visible(environment)
        }
        bindings: _Bindings = {}
        for name in names:
            alternatives = frozenset().union(
                *(
                    self._visible(environment).get(name, frozenset())
                    for environment in reachable
                )
            )
            if alternatives:
                bindings[name] = alternatives
        return _Environment(scope, bindings)

    def _flow_join(self, *flows: _Flow) -> _Flow:
        return _Flow(
            normal=self._join(*(flow.normal for flow in flows)),
            breaks=self._join(*(flow.breaks for flow in flows)),
            continues=self._join(*(flow.continues for flow in flows)),
            exceptions=self._join(*(flow.exceptions for flow in flows)),
        )

    def _statements(
        self,
        statements: list[ast.stmt],
        environment: _Environment,
        *,
        class_scope: bool,
    ) -> _Flow:
        flow = _Flow(normal=environment)
        for statement in statements:
            current = flow.normal
            if current is None:
                break
            if getattr(statement, "type_comment", None) is not None:
                self._issues.add(
                    f"line {statement.lineno}: public type comments are unsupported"
                )
            result = self._statement(statement, current, class_scope=class_scope)
            flow = _Flow(
                normal=result.normal,
                breaks=self._join(flow.breaks, result.breaks),
                continues=self._join(flow.continues, result.continues),
                # Every statement-prefix is a conservative handler edge.
                exceptions=self._join(
                    flow.exceptions, current, result.exceptions
                ),
            )
        return flow

    def _statement(
        self,
        statement: ast.stmt,
        environment: _Environment,
        *,
        class_scope: bool,
    ) -> _Flow:
        if isinstance(statement, ast.Import):
            replacements: _Bindings = {}
            for imported in statement.names:
                name = imported.asname or imported.name.split(".", maxsplit=1)[0]
                kind = (
                    "typing-module"
                    if imported.name in {"typing", "typing_extensions"}
                    else "other"
                )
                replacements[name] = frozenset({_Binding(kind)})
            return _Flow(normal=self._replace(environment, replacements))

        if isinstance(statement, ast.ImportFrom):
            replacements = {}
            for imported in statement.names:
                name = imported.asname or imported.name
                if imported.name == "*":
                    self._issues.add(
                        f"line {statement.lineno}: star imports have ambiguous type provenance"
                    )
                    continue
                if statement.module in {"typing", "typing_extensions"}:
                    kind = _TYPING_KINDS.get(imported.name, "other")
                elif statement.module == "builtins":
                    kind = _BUILTIN_KINDS.get(imported.name, "other")
                else:
                    kind = "other"
                replacements[name] = frozenset({_Binding(kind)})
                if kind == "any":
                    self._issues.add(f"line {statement.lineno}: imports Any")
            return _Flow(normal=self._replace(environment, replacements))

        if isinstance(statement, (ast.Assign, ast.AnnAssign)):
            value = statement.value
            if isinstance(statement, ast.AnnAssign):
                self._add_root(
                    "annotated assignment",
                    statement.annotation,
                    environment,
                    "type",
                )
                annotation_kinds = self._possible_kinds(
                    statement.annotation,
                    self._visible(environment),
                    environment.scope,
                    set(),
                )
                if value is not None and "type-alias" in annotation_kinds:
                    self._add_root(
                        "explicit type alias", value, environment, "type"
                    )
            if value is not None:
                self._add_root(
                    "declaration assignment value",
                    value,
                    environment,
                    "declaration",
                )
            targets = (
                statement.targets
                if isinstance(statement, ast.Assign)
                else [statement.target]
            )
            replacements = {}
            for target in targets:
                for name in _assignment_names(target):
                    binding = (
                        self._definition_binding(value, environment)
                        if value is not None and isinstance(target, ast.Name)
                        else _OTHER
                    )
                    replacements[name] = frozenset({binding})
            return _Flow(normal=self._replace(environment, replacements))

        if isinstance(statement, ast.AugAssign):
            self._add_root(
                "augmented assignment value",
                statement.value,
                environment,
                "value",
            )
            replacements = {
                name: frozenset({_OTHER})
                for name in _assignment_names(statement.target)
            }
            return _Flow(normal=self._replace(environment, replacements))

        if type(statement).__name__ == "TypeAlias":
            declaration_environment = self._type_parameter_environment(
                statement, environment
            )
            for expression in _pep695_roots(statement):
                self._add_root(
                    "PEP 695 type alias",
                    expression,
                    declaration_environment,
                    "type",
                )
            name = _pep695_alias_name(statement)
            value = getattr(statement, "value", None)
            if name is None or not isinstance(value, ast.expr):
                return _Flow(normal=environment)
            binding = self._definition_binding(value, declaration_environment)
            return _Flow(
                normal=self._replace(
                    environment, {name: frozenset({binding})}
                )
            )

        if isinstance(statement, ast.ClassDef):
            declaration_environment = self._type_parameter_environment(
                statement, environment
            )
            for expression in _pep695_roots(statement):
                self._add_root(
                    "class type parameter",
                    expression,
                    declaration_environment,
                    "type",
                )
            for base in statement.bases:
                self._add_root("class base", base, declaration_environment, "type")
            for keyword in statement.keywords:
                if keyword.arg is None:
                    context = "class-keyword-unpack"
                    label = "class keyword unpack"
                elif keyword.arg == "metaclass":
                    context = "type"
                    label = "class metaclass"
                elif keyword.arg == "extra_items":
                    context = "typed-dict-extra-items"
                    label = "TypedDict extra_items"
                else:
                    context = "value"
                    label = "class configuration"
                self._add_root(
                    label,
                    keyword.value,
                    declaration_environment,
                    context,
                    type_candidates=tuple(statement.bases),
                )

            parent_bindings = self._visible(declaration_environment)
            child_scope = self._class_scopes.get(id(statement))
            if child_scope is None:
                child_scope = _Scope(environment.scope, parent_bindings)
                self._class_scopes[id(statement)] = child_scope
                self._version += 1
            elif _merge_bindings(child_scope.parent_bindings, parent_bindings):
                self._version += 1
            child_flow = self._statements(
                statement.body,
                _Environment(child_scope, {}),
                class_scope=True,
            )
            if child_flow.normal is not None:
                self._update_scope_final(child_scope, child_flow.normal.bindings)
            result = self._replace(
                environment,
                {statement.name: frozenset({_OTHER})},
            )
            # A class body failure resumes in the enclosing scope without
            # exposing the body's transient local namespace.  The enclosing
            # sequence already contributes its pre-class prefix state to the
            # exceptional edge.
            return _Flow(normal=result)

        if isinstance(statement, (ast.FunctionDef, ast.AsyncFunctionDef)):
            declaration_environment = self._type_parameter_environment(
                statement, environment
            )
            for expression in _pep695_roots(statement):
                self._add_root(
                    "function type parameter",
                    expression,
                    declaration_environment,
                    "type",
                )
            parameters = [
                *statement.args.posonlyargs,
                *statement.args.args,
                *statement.args.kwonlyargs,
            ]
            if statement.args.vararg is not None:
                parameters.append(statement.args.vararg)
            if statement.args.kwarg is not None:
                parameters.append(statement.args.kwarg)
            positional = [*statement.args.posonlyargs, *statement.args.args]
            receiver = (
                class_scope
                and not _is_static_method(statement)
                and bool(positional)
                and positional[0].arg in {"self", "cls"}
            )
            for index, parameter in enumerate(parameters):
                if parameter.annotation is None:
                    if not (receiver and index == 0):
                        self._issues.add(
                            f"line {parameter.lineno}: {statement.name}."
                            f"{parameter.arg} has no annotation"
                        )
                    continue
                context = (
                    "protocol-object"
                    if statement.name == "__eq__"
                    and index == 1
                    and isinstance(parameter.annotation, ast.Name)
                    and parameter.annotation.id == "object"
                    else "type"
                )
                self._add_root(
                    f"{statement.name}.{parameter.arg} annotation",
                    parameter.annotation,
                    declaration_environment,
                    context,
                )
            if statement.returns is None:
                self._issues.add(
                    f"line {statement.lineno}: {statement.name} has no return"
                )
            else:
                self._add_root(
                    f"{statement.name} return",
                    statement.returns,
                    declaration_environment,
                    "type",
                )
            for default in [
                *statement.args.defaults,
                *(
                    default
                    for default in statement.args.kw_defaults
                    if default is not None
                ),
            ]:
                self._add_root(
                    f"{statement.name} default", default, environment, "value"
                )
            result = self._replace(
                environment,
                {statement.name: frozenset({_OTHER})},
            )
            return _Flow(normal=result)

        if isinstance(statement, ast.If):
            truth = _literal_truth(statement.test)
            if truth is True:
                return self._statements(
                    statement.body, environment, class_scope=class_scope
                )
            if truth is False:
                return self._statements(
                    statement.orelse, environment, class_scope=class_scope
                )
            body = self._statements(
                statement.body, environment, class_scope=class_scope
            )
            otherwise = self._statements(
                statement.orelse, environment, class_scope=class_scope
            )
            return self._flow_join(body, otherwise)

        if isinstance(statement, (ast.For, ast.AsyncFor, ast.While)):
            return self._loop(statement, environment, class_scope=class_scope)

        if isinstance(statement, (ast.Try, ast.TryStar)):
            return self._try(statement, environment, class_scope=class_scope)

        if isinstance(statement, ast.Match):
            outcomes: list[_Flow] = []
            fallthrough: _Environment | None = environment
            for case in statement.cases:
                if fallthrough is None:
                    break
                guard = _literal_truth(case.guard) if case.guard is not None else True
                case_environment = self._replace(
                    fallthrough,
                    {
                        name: frozenset({_OTHER})
                        for name in _pattern_names(case.pattern)
                    },
                )
                if guard is not False:
                    outcomes.append(
                        self._statements(
                            case.body,
                            case_environment,
                            class_scope=class_scope,
                        )
                    )
                if (
                    case.guard is None or guard is True
                ) and _pattern_is_irrefutable(case.pattern):
                    fallthrough = None
            if fallthrough is not None:
                outcomes.append(_Flow(normal=fallthrough))
            return self._flow_join(*outcomes) if outcomes else _Flow()

        if isinstance(statement, (ast.With, ast.AsyncWith)):
            replacements = {
                name: frozenset({_OTHER})
                for item in statement.items
                if item.optional_vars is not None
                for name in _assignment_names(item.optional_vars)
            }
            body_environment = self._replace(environment, replacements)
            return self._statements(
                statement.body, body_environment, class_scope=class_scope
            )

        if isinstance(statement, ast.Delete):
            names = {
                name
                for target in statement.targets
                for name in _assignment_names(target)
            }
            return _Flow(normal=self._delete(environment, names))

        if isinstance(statement, ast.Break):
            return _Flow(breaks=environment)
        if isinstance(statement, ast.Continue):
            return _Flow(continues=environment)
        if isinstance(statement, (ast.Return, ast.Raise)):
            return _Flow(exceptions=environment if isinstance(statement, ast.Raise) else None)

        return _Flow(normal=environment)

    def _loop(
        self,
        statement: ast.For | ast.AsyncFor | ast.While,
        environment: _Environment,
        *,
        class_scope: bool,
    ) -> _Flow:
        if isinstance(statement, ast.While):
            truth = _literal_truth(statement.test)
            if truth is False:
                return self._statements(
                    statement.orelse, environment, class_scope=class_scope
                )
        else:
            truth = None

        header = environment
        accumulated_breaks: _Environment | None = None
        accumulated_exceptions: _Environment | None = None
        while True:
            version = self._version
            body_environment = header
            if isinstance(statement, (ast.For, ast.AsyncFor)):
                body_environment = self._replace(
                    body_environment,
                    {
                        name: frozenset({_OTHER})
                        for name in _assignment_names(statement.target)
                    },
                )
            body = self._statements(
                statement.body, body_environment, class_scope=class_scope
            )
            accumulated_breaks = self._join(accumulated_breaks, body.breaks)
            accumulated_exceptions = self._join(
                accumulated_exceptions, body.exceptions
            )
            back_edge = self._join(body.normal, body.continues)
            new_header = self._join(environment, back_edge)
            if new_header is None:
                new_header = environment
            stable = (
                _binding_maps_equal(
                    self._visible(new_header), self._visible(header)
                )
                and self._version == version
            )
            header = new_header
            if stable:
                break

        exhaustion = None if truth is True else header
        if exhaustion is not None:
            normal_exit = self._statements(
                statement.orelse, exhaustion, class_scope=class_scope
            )
        else:
            normal_exit = _Flow()
        return _Flow(
            normal=self._join(normal_exit.normal, accumulated_breaks),
            breaks=normal_exit.breaks,
            continues=normal_exit.continues,
            exceptions=self._join(
                accumulated_exceptions, normal_exit.exceptions
            ),
        )

    def _try(
        self,
        statement: ast.Try | ast.TryStar,
        environment: _Environment,
        *,
        class_scope: bool,
    ) -> _Flow:
        body = self._statements(
            statement.body, environment, class_scope=class_scope
        )
        normal = body.normal
        else_flow = (
            self._statements(statement.orelse, normal, class_scope=class_scope)
            if normal is not None
            else _Flow()
        )
        handler_flows: list[_Flow] = []
        catches_all = False
        if body.exceptions is not None:
            for handler in statement.handlers:
                handler_environment = body.exceptions
                if handler.type is None:
                    catches_all = True
                else:
                    self._add_root(
                        "exception handler type",
                        handler.type,
                        handler_environment,
                        "type",
                    )
                if handler.name is not None:
                    handler_environment = self._replace(
                        handler_environment,
                        {handler.name: frozenset({_OTHER})},
                    )
                handler_flow = self._statements(
                    handler.body,
                    handler_environment,
                    class_scope=class_scope,
                )
                if handler.name is not None:
                    handler_flow = _Flow(
                        normal=(
                            self._delete(handler_flow.normal, {handler.name})
                            if handler_flow.normal is not None
                            else None
                        ),
                        breaks=(
                            self._delete(handler_flow.breaks, {handler.name})
                            if handler_flow.breaks is not None
                            else None
                        ),
                        continues=(
                            self._delete(handler_flow.continues, {handler.name})
                            if handler_flow.continues is not None
                            else None
                        ),
                        exceptions=(
                            self._delete(handler_flow.exceptions, {handler.name})
                            if handler_flow.exceptions is not None
                            else None
                        ),
                    )
                handler_flows.append(handler_flow)
        handlers = self._flow_join(*handler_flows) if handler_flows else _Flow()
        combined = _Flow(
            normal=self._join(else_flow.normal, handlers.normal),
            breaks=self._join(body.breaks, else_flow.breaks, handlers.breaks),
            continues=self._join(
                body.continues, else_flow.continues, handlers.continues
            ),
            exceptions=self._join(
                else_flow.exceptions,
                handlers.exceptions,
                None if catches_all else body.exceptions,
            ),
        )
        if not statement.finalbody:
            return combined
        return self._apply_finally(
            statement.finalbody, combined, class_scope=class_scope
        )

    def _apply_finally(
        self,
        statements: list[ast.stmt],
        flow: _Flow,
        *,
        class_scope: bool,
    ) -> _Flow:
        result = _Flow()
        for channel, incoming in (
            ("normal", flow.normal),
            ("break", flow.breaks),
            ("continue", flow.continues),
            ("exception", flow.exceptions),
        ):
            if incoming is None:
                continue
            final = self._statements(
                statements, incoming, class_scope=class_scope
            )
            normal = result.normal
            breaks = self._join(result.breaks, final.breaks)
            continues = self._join(result.continues, final.continues)
            exceptions = self._join(result.exceptions, final.exceptions)
            if channel == "normal":
                normal = self._join(normal, final.normal)
            elif channel == "break":
                breaks = self._join(breaks, final.normal)
            elif channel == "continue":
                continues = self._join(continues, final.normal)
            else:
                exceptions = self._join(exceptions, final.normal)
            result = _Flow(normal, breaks, continues, exceptions)
        return result

    def _type_parameter_environment(
        self, declaration: ast.AST, environment: _Environment
    ) -> _Environment:
        names = _type_parameter_names(declaration)
        if not names:
            return environment
        return self._replace(
            environment,
            {name: frozenset({_OTHER}) for name in names},
        )

    def _possible_kinds(
        self,
        expression: ast.expr,
        captures: _Bindings,
        scope: _Scope,
        visiting: set[int],
    ) -> set[str]:
        if isinstance(expression, ast.Constant) and isinstance(
            expression.value, str
        ):
            try:
                nested = ast.parse(expression.value, mode="eval").body
            except SyntaxError:
                return {"other"}
            return self._possible_kinds(nested, captures, scope, visiting)
        if isinstance(expression, ast.Name):
            kinds: set[str] = set()
            for binding in self._lookup(expression.id, captures, scope):
                if binding.kind != "expression":
                    kinds.add(binding.kind)
                    continue
                definition = self._definition_for(binding)
                if definition is None or id(definition) in visiting:
                    kinds.add("other")
                    continue
                visiting.add(id(definition))
                kinds.update(
                    self._possible_kinds(
                        definition.expression,
                        definition.captures,
                        definition.scope,
                        visiting,
                    )
                )
                visiting.remove(id(definition))
            return kinds or {"other"}
        if isinstance(expression, ast.Attribute):
            module_kinds = self._possible_kinds(
                expression.value, captures, scope, visiting
            )
            kinds = {"other"} if module_kinds - {"typing-module"} else set()
            if "typing-module" in module_kinds:
                kinds.add(_TYPING_KINDS.get(expression.attr, "other"))
            return kinds or {"other"}
        if isinstance(expression, ast.IfExp):
            return self._possible_kinds(
                expression.body, captures, scope, visiting
            ) | self._possible_kinds(
                expression.orelse, captures, scope, visiting
            )
        return {"other"}

    def _scan(
        self,
        expression: ast.expr,
        captures: _Bindings,
        scope: _Scope,
        context: str,
        visiting: set[tuple[int, str]],
        strings: set[_ForwardStringKey],
    ) -> set[_Problem]:
        if context in {"value", "metadata", "field-name"}:
            return set()
        if context == "protocol-object":
            return set()
        if context == "declaration":
            return self._scan_declaration(
                expression, captures, scope, visiting, strings
            )
        if context == "named-tuple-fields":
            return self._scan_named_fields(
                expression, captures, scope, visiting, strings
            )
        if context == "named-tuple-field":
            return self._scan_named_field(
                expression, captures, scope, visiting, strings
            )
        if context == "typed-dict-fields":
            return self._scan_typed_fields(
                expression, captures, scope, visiting, strings
            )
        return self._scan_type(
            expression, captures, scope, visiting, strings
        )

    def _scan_declaration(
        self,
        expression: ast.expr,
        captures: _Bindings,
        scope: _Scope,
        visiting: set[tuple[int, str]],
        strings: set[_ForwardStringKey],
    ) -> set[_Problem]:
        if isinstance(expression, ast.Name):
            # A bare assignment is not itself a type declaration.  Reject a
            # direct imported escape hatch, but leave arbitrary forward value
            # aliases alone until a type-bearing use proves their context.
            if {
                binding.kind
                for binding in self._lookup(expression.id, captures, scope)
            } & {"any", "catch-all"}:
                return self._scan_type(
                    expression, captures, scope, visiting, strings
                )
            return set()
        if isinstance(expression, ast.Attribute):
            if self._possible_kinds(expression, captures, scope, set()) & {
                "any",
                "catch-all",
            }:
                return self._scan_type(
                    expression, captures, scope, visiting, strings
                )
            return set()
        if isinstance(expression, ast.Subscript) or (
            isinstance(expression, ast.BinOp)
            and isinstance(expression.op, ast.BitOr)
        ):
            return self._scan_type(
                expression, captures, scope, visiting, strings
            )
        if isinstance(expression, ast.Call):
            return self._scan_call(
                expression, captures, scope, visiting, strings
            )
        return set()

    def _scan_type(
        self,
        expression: ast.expr,
        captures: _Bindings,
        scope: _Scope,
        visiting: set[tuple[int, str]],
        strings: set[_ForwardStringKey],
    ) -> set[_Problem]:
        if isinstance(expression, ast.Name):
            problems: set[_Problem] = set()
            for binding in self._lookup(expression.id, captures, scope):
                if binding.kind == "any":
                    problems.add(_Problem("uses Any", expression.lineno))
                elif binding.kind == "catch-all":
                    problems.add(
                        _Problem("uses bare object as a catch-all", expression.lineno)
                    )
                elif binding.kind == "expression":
                    definition = self._definition_for(binding)
                    if definition is None:
                        continue
                    key = (id(definition), "type")
                    if key in visiting:
                        continue
                    visiting.add(key)
                    problems.update(
                        self._scan_type(
                            definition.expression,
                            definition.captures,
                            definition.scope,
                            visiting,
                            strings,
                        )
                    )
                    visiting.remove(key)
            return problems
        if isinstance(expression, ast.Attribute):
            kinds = self._possible_kinds(expression, captures, scope, set())
            problems = set()
            if "any" in kinds:
                problems.add(_Problem("uses Any", expression.lineno))
            if "catch-all" in kinds:
                problems.add(
                    _Problem("uses a catch-all type", expression.lineno)
                )
            return problems
        if isinstance(expression, ast.Constant):
            if not isinstance(expression.value, str):
                return set()
            string_key = (
                expression.value,
                id(scope),
                frozenset(captures.items()),
            )
            if string_key in strings:
                return set()
            strings.add(string_key)
            try:
                try:
                    nested = ast.parse(expression.value, mode="eval").body
                except SyntaxError:
                    return {
                        _Problem(
                            "has an unsupported forward-annotation string",
                            expression.lineno,
                        )
                    }
                return self._scan_type(
                    nested, captures, scope, visiting, strings
                )
            finally:
                # Identical text under a different captured environment is a
                # different abstract value.  Track only the active recursive
                # string path so cycles terminate without suppressing sibling
                # definitions that have distinct provenance.
                strings.remove(string_key)
        if isinstance(expression, ast.Subscript):
            problems = self._scan_type(
                expression.value, captures, scope, visiting, strings
            )
            arguments = (
                list(expression.slice.elts)
                if isinstance(expression.slice, ast.Tuple)
                else [expression.slice]
            )
            kinds = self._possible_kinds(
                expression.value, captures, scope, set()
            )
            for index, argument in enumerate(arguments):
                contexts: set[str] = set()
                for kind in kinds:
                    if kind == "literal":
                        contexts.add("value")
                    elif kind == "annotated":
                        contexts.add("type" if index == 0 else "metadata")
                    else:
                        contexts.add("type")
                if "type" in contexts:
                    problems.update(
                        self._scan_type(
                            argument, captures, scope, visiting, strings
                        )
                    )
            return problems
        if isinstance(expression, ast.Call):
            problems = self._scan_call(
                expression, captures, scope, visiting, strings
            )
            if self._possible_kinds(expression.func, captures, scope, set()) == {
                "other"
            }:
                problems.add(
                    _Problem("has an unsupported dynamic type call", expression.lineno)
                )
            return problems
        if isinstance(expression, (ast.List, ast.Tuple, ast.Set)):
            problems = set()
            for element in expression.elts:
                problems.update(
                    self._scan_type(
                        element, captures, scope, visiting, strings
                    )
                )
            return problems
        if isinstance(expression, ast.Dict):
            return {
                _Problem("has an unsupported dynamic type mapping", expression.lineno)
            }
        if isinstance(expression, ast.IfExp):
            return self._scan_type(
                expression.body, captures, scope, visiting, strings
            ) | self._scan_type(
                expression.orelse, captures, scope, visiting, strings
            )
        if isinstance(expression, ast.Starred):
            return self._scan_type(
                expression.value, captures, scope, visiting, strings
            )
        if isinstance(expression, ast.BinOp):
            return self._scan_type(
                expression.left, captures, scope, visiting, strings
            ) | self._scan_type(
                expression.right, captures, scope, visiting, strings
            )
        if isinstance(expression, ast.UnaryOp):
            return self._scan_type(
                expression.operand, captures, scope, visiting, strings
            )
        return set()

    def _scan_call(
        self,
        expression: ast.Call,
        captures: _Bindings,
        scope: _Scope,
        visiting: set[tuple[int, str]],
        strings: set[_ForwardStringKey],
    ) -> set[_Problem]:
        kinds = self._possible_kinds(expression.func, captures, scope, set())
        problems: set[_Problem] = set()
        special_kinds = kinds - {"other"}
        if not special_kinds:
            return problems

        shapes, supported = self._expand_call_shapes(
            expression, captures, scope, set()
        )
        if not supported:
            problems.add(_unsupported_invocation(expression))

        for kind in kinds - {"other"}:
            if kind in {"any", "catch-all"}:
                problems.update(
                    self._scan_type(
                        expression.func, captures, scope, visiting, strings
                    )
                )
                continue
            for shape in shapes:
                problems.update(
                    self._scan_call_shape(
                        kind, shape, visiting=visiting, strings=strings
                    )
                )
        return problems

    def _scan_call_shape(
        self,
        kind: str,
        shape: _CallShape,
        *,
        visiting: set[tuple[int, str]],
        strings: set[_ForwardStringKey],
    ) -> set[_Problem]:
        problems: set[_Problem] = set()

        def scan_type(argument: _LocatedExpression) -> None:
            problems.update(
                self._scan_type(
                    argument.expression,
                    argument.captures,
                    argument.scope,
                    visiting,
                    strings,
                )
            )

        def scan_named_fields(argument: _LocatedExpression) -> None:
            problems.update(
                self._scan_named_fields(
                    argument.expression,
                    argument.captures,
                    argument.scope,
                    visiting,
                    strings,
                )
            )

        def scan_typed_fields(argument: _LocatedExpression) -> None:
            problems.update(
                self._scan_typed_fields(
                    argument.expression,
                    argument.captures,
                    argument.scope,
                    visiting,
                    strings,
                )
            )

        if kind in {"type-alias-type", "new-type"}:
            for argument in shape.positional[1:2]:
                scan_type(argument)
            type_keywords = {"tp", "value", "type_params"}
        elif kind == "type-var":
            for argument in shape.positional[1:]:
                scan_type(argument)
            type_keywords = {"bound", "default"}
        elif kind in {"param-spec", "type-var-tuple"}:
            type_keywords = {"bound", "default"}
        elif kind == "forward-ref":
            for argument in shape.positional[:1]:
                scan_type(argument)
            type_keywords = {"arg"}
        elif kind == "named-tuple":
            for argument in shape.positional[1:2]:
                scan_named_fields(argument)
            for name, argument in shape.keywords:
                if name == "fields":
                    scan_named_fields(argument)
                elif name not in {
                    "typename",
                    "rename",
                    "defaults",
                    "module",
                }:
                    scan_type(argument)
            return problems
        elif kind == "typed-dict":
            for argument in shape.positional[1:2]:
                scan_typed_fields(argument)
            for name, argument in shape.keywords:
                if name == "fields":
                    scan_typed_fields(argument)
                elif name == "extra_items" or name not in {
                    "typename",
                    "total",
                    "closed",
                    "module",
                }:
                    scan_type(argument)
            return problems
        elif kind == "cast":
            for argument in shape.positional[:1]:
                scan_type(argument)
            type_keywords = {"typ", "type"}
        elif kind == "assert-type":
            for argument in shape.positional[1:2]:
                scan_type(argument)
            type_keywords = {"typ", "type"}
        else:
            return problems

        for name, argument in shape.keywords:
            if name in type_keywords:
                scan_type(argument)
        return problems

    def _expand_call_shapes(
        self,
        expression: ast.Call,
        captures: _Bindings,
        scope: _Scope,
        visiting: set[tuple[int, str]],
    ) -> tuple[list[_CallShape], bool]:
        shapes = [_CallShape()]
        for argument in expression.args:
            if isinstance(argument, ast.Starred):
                alternatives, supported = self._expand_sequence(
                    argument.value, captures, scope, visiting
                )
                if not supported:
                    return shapes, False
                shapes = [
                    _CallShape(shape.positional + alternative, shape.keywords)
                    for shape in shapes
                    for alternative in alternatives
                ]
            else:
                located = _LocatedExpression(argument, captures, scope)
                shapes = [
                    _CallShape(shape.positional + (located,), shape.keywords)
                    for shape in shapes
                ]
        for keyword in expression.keywords:
            if keyword.arg is None:
                alternatives, supported = self._expand_mapping(
                    keyword.value, captures, scope, visiting
                )
                if not supported:
                    return shapes, False
                shapes = [
                    _CallShape(shape.positional, shape.keywords + alternative)
                    for shape in shapes
                    for alternative in alternatives
                ]
            else:
                item = (
                    keyword.arg,
                    _LocatedExpression(keyword.value, captures, scope),
                )
                shapes = [
                    _CallShape(shape.positional, shape.keywords + (item,))
                    for shape in shapes
                ]
        return shapes, True

    def _expand_sequence(
        self,
        expression: ast.expr,
        captures: _Bindings,
        scope: _Scope,
        visiting: set[tuple[int, str]],
    ) -> tuple[list[tuple[_LocatedExpression, ...]], bool]:
        key = (id(expression), "call-sequence")
        if key in visiting:
            return [], False
        visiting.add(key)
        try:
            if isinstance(expression, ast.Name):
                alternatives, supported = self._expression_alternatives(
                    expression, captures, scope
                )
                expanded: list[tuple[_LocatedExpression, ...]] = []
                for (
                    alternative,
                    alternative_captures,
                    alternative_scope,
                ) in alternatives:
                    nested, nested_supported = self._expand_sequence(
                        alternative,
                        alternative_captures,
                        alternative_scope,
                        visiting,
                    )
                    expanded.extend(nested)
                    supported = supported and nested_supported
                return expanded, supported and bool(alternatives)
            if isinstance(expression, (ast.List, ast.Tuple)):
                shapes: list[tuple[_LocatedExpression, ...]] = [()]
                for element in expression.elts:
                    if isinstance(element, ast.Starred):
                        alternatives, supported = self._expand_sequence(
                            element.value, captures, scope, visiting
                        )
                        if not supported:
                            return shapes, False
                    else:
                        alternatives = [
                            (_LocatedExpression(element, captures, scope),)
                        ]
                    shapes = [
                        shape + alternative
                        for shape in shapes
                        for alternative in alternatives
                    ]
                return shapes, True
            if isinstance(expression, ast.IfExp):
                body, body_supported = self._expand_sequence(
                    expression.body, captures, scope, visiting
                )
                otherwise, otherwise_supported = self._expand_sequence(
                    expression.orelse, captures, scope, visiting
                )
                return body + otherwise, body_supported and otherwise_supported
            if isinstance(expression, ast.BinOp) and isinstance(
                expression.op, ast.Add
            ):
                left, left_supported = self._expand_sequence(
                    expression.left, captures, scope, visiting
                )
                right, right_supported = self._expand_sequence(
                    expression.right, captures, scope, visiting
                )
                return (
                    [
                        left_shape + right_shape
                        for left_shape in left
                        for right_shape in right
                    ],
                    left_supported and right_supported,
                )
            if isinstance(expression, ast.Call):
                kinds = self._possible_kinds(
                    expression.func, captures, scope, set()
                )
                if kinds and kinds <= {"builtin-list", "builtin-tuple"}:
                    if not expression.args and not expression.keywords:
                        return [()], True
                    if len(expression.args) == 1 and not expression.keywords:
                        return self._expand_sequence(
                            expression.args[0], captures, scope, visiting
                        )
            return [], False
        finally:
            visiting.remove(key)

    def _expand_mapping(
        self,
        expression: ast.expr,
        captures: _Bindings,
        scope: _Scope,
        visiting: set[tuple[int, str]],
    ) -> tuple[list[tuple[tuple[str, _LocatedExpression], ...]], bool]:
        key = (id(expression), "call-mapping")
        if key in visiting:
            return [], False
        visiting.add(key)
        try:
            if isinstance(expression, ast.Name):
                alternatives, supported = self._expression_alternatives(
                    expression, captures, scope
                )
                expanded: list[tuple[tuple[str, _LocatedExpression], ...]] = []
                for (
                    alternative,
                    alternative_captures,
                    alternative_scope,
                ) in alternatives:
                    nested, nested_supported = self._expand_mapping(
                        alternative,
                        alternative_captures,
                        alternative_scope,
                        visiting,
                    )
                    expanded.extend(nested)
                    supported = supported and nested_supported
                return expanded, supported and bool(alternatives)
            if isinstance(expression, ast.Dict):
                shapes: list[tuple[tuple[str, _LocatedExpression], ...]] = [()]
                for item_key, value in zip(
                    expression.keys, expression.values, strict=True
                ):
                    if item_key is None:
                        alternatives, supported = self._expand_mapping(
                            value, captures, scope, visiting
                        )
                        if not supported:
                            return shapes, False
                    elif isinstance(item_key, ast.Constant) and isinstance(
                        item_key.value, str
                    ):
                        alternatives = [
                            (
                                (
                                    item_key.value,
                                    _LocatedExpression(value, captures, scope),
                                ),
                            )
                        ]
                    else:
                        return shapes, False
                    shapes = [
                        shape + alternative
                        for shape in shapes
                        for alternative in alternatives
                    ]
                return shapes, True
            if isinstance(expression, ast.IfExp):
                body, body_supported = self._expand_mapping(
                    expression.body, captures, scope, visiting
                )
                otherwise, otherwise_supported = self._expand_mapping(
                    expression.orelse, captures, scope, visiting
                )
                return body + otherwise, body_supported and otherwise_supported
            if isinstance(expression, ast.BinOp) and isinstance(
                expression.op, ast.BitOr
            ):
                left, left_supported = self._expand_mapping(
                    expression.left, captures, scope, visiting
                )
                right, right_supported = self._expand_mapping(
                    expression.right, captures, scope, visiting
                )
                return (
                    [
                        left_shape + right_shape
                        for left_shape in left
                        for right_shape in right
                    ],
                    left_supported and right_supported,
                )
            if isinstance(expression, ast.Call):
                kinds = self._possible_kinds(
                    expression.func, captures, scope, set()
                )
                if kinds and kinds <= {"builtin-dict"}:
                    shapes: list[tuple[tuple[str, _LocatedExpression], ...]] = [()]
                    for argument in expression.args:
                        if isinstance(argument, ast.Starred):
                            return shapes, False
                        alternatives, supported = self._expand_mapping(
                            argument, captures, scope, visiting
                        )
                        if not supported:
                            return shapes, False
                        shapes = [
                            shape + alternative
                            for shape in shapes
                            for alternative in alternatives
                        ]
                    for keyword in expression.keywords:
                        if keyword.arg is None:
                            alternatives, supported = self._expand_mapping(
                                keyword.value, captures, scope, visiting
                            )
                            if not supported:
                                return shapes, False
                        else:
                            alternatives = [
                                (
                                    (
                                        keyword.arg,
                                        _LocatedExpression(
                                            keyword.value, captures, scope
                                        ),
                                    ),
                                )
                            ]
                        shapes = [
                            shape + alternative
                            for shape in shapes
                            for alternative in alternatives
                        ]
                    return shapes, True
            return [], False
        finally:
            visiting.remove(key)

    def _scan_class_keyword_unpack(
        self,
        expression: ast.expr,
        captures: _Bindings,
        scope: _Scope,
        *,
        visiting: set[tuple[int, str]],
        strings: set[_ForwardStringKey],
    ) -> set[_Problem]:
        mappings, supported = self._expand_mapping(
            expression, captures, scope, set()
        )
        if not supported:
            return {_unsupported_class_keyword_unpack(expression)}
        problems: set[_Problem] = set()
        for mapping in mappings:
            for name, argument in mapping:
                if name in {"metaclass", "extra_items"}:
                    problems.update(
                        self._scan_type(
                            argument.expression,
                            argument.captures,
                            argument.scope,
                            visiting,
                            strings,
                        )
                    )
        return problems

    def _expression_alternatives(
        self,
        expression: ast.expr,
        captures: _Bindings,
        scope: _Scope,
    ) -> tuple[list[tuple[ast.expr, _Bindings, _Scope]], bool]:
        if not isinstance(expression, ast.Name):
            return [(expression, captures, scope)], True
        alternatives: list[tuple[ast.expr, _Bindings, _Scope]] = []
        supported = True
        bindings = self._lookup(expression.id, captures, scope)
        if not bindings:
            return [], False
        for binding in bindings:
            definition = self._definition_for(binding)
            if binding.kind != "expression" or definition is None:
                supported = False
                continue
            alternatives.append(
                (definition.expression, definition.captures, definition.scope)
            )
        return alternatives, supported

    def _scan_named_fields(
        self,
        expression: ast.expr,
        captures: _Bindings,
        scope: _Scope,
        visiting: set[tuple[int, str]],
        strings: set[_ForwardStringKey],
    ) -> set[_Problem]:
        if isinstance(expression, ast.Name):
            key = (id(expression), "named-tuple-fields")
            if key in visiting:
                return set()
            visiting.add(key)
            alternatives, supported = self._expression_alternatives(
                expression, captures, scope
            )
            problems = set()
            for alternative, alternative_captures, alternative_scope in alternatives:
                problems.update(
                    self._scan_named_fields(
                        alternative,
                        alternative_captures,
                        alternative_scope,
                        visiting,
                        strings,
                    )
                )
            visiting.remove(key)
            if not supported:
                problems.add(_unsupported_fields("NamedTuple", expression))
            return problems
        if isinstance(expression, (ast.List, ast.Tuple)):
            problems = set()
            for element in expression.elts:
                if isinstance(element, ast.Starred):
                    problems.update(
                        self._scan_named_fields(
                            element.value, captures, scope, visiting, strings
                        )
                    )
                else:
                    problems.update(
                        self._scan_named_field(
                            element, captures, scope, visiting, strings
                        )
                    )
            return problems
        if isinstance(expression, ast.BinOp) and isinstance(expression.op, ast.Add):
            return self._scan_named_fields(
                expression.left, captures, scope, visiting, strings
            ) | self._scan_named_fields(
                expression.right, captures, scope, visiting, strings
            )
        if isinstance(expression, ast.BinOp) and isinstance(expression.op, ast.Mult):
            if _literal_integer(expression.left) is not None:
                return self._scan_named_fields(
                    expression.right, captures, scope, visiting, strings
                )
            if _literal_integer(expression.right) is not None:
                return self._scan_named_fields(
                    expression.left, captures, scope, visiting, strings
                )
            return {_unsupported_fields("NamedTuple", expression)}
        if isinstance(expression, ast.IfExp):
            return self._scan_named_fields(
                expression.body, captures, scope, visiting, strings
            ) | self._scan_named_fields(
                expression.orelse, captures, scope, visiting, strings
            )
        if isinstance(expression, ast.Call):
            kinds = self._possible_kinds(expression.func, captures, scope, set())
            if kinds and kinds <= {"builtin-list", "builtin-tuple"}:
                if not expression.args:
                    return set()
                if len(expression.args) == 1 and not expression.keywords:
                    return self._scan_named_fields(
                        expression.args[0], captures, scope, visiting, strings
                    )
            return {_unsupported_fields("NamedTuple", expression)}
        return {_unsupported_fields("NamedTuple", expression)}

    def _scan_named_field(
        self,
        expression: ast.expr,
        captures: _Bindings,
        scope: _Scope,
        visiting: set[tuple[int, str]],
        strings: set[_ForwardStringKey],
    ) -> set[_Problem]:
        if isinstance(expression, ast.Name):
            key = (id(expression), "named-tuple-field")
            if key in visiting:
                return set()
            visiting.add(key)
            alternatives, supported = self._expression_alternatives(
                expression, captures, scope
            )
            problems = set()
            for alternative, alternative_captures, alternative_scope in alternatives:
                problems.update(
                    self._scan_named_field(
                        alternative,
                        alternative_captures,
                        alternative_scope,
                        visiting,
                        strings,
                    )
                )
            visiting.remove(key)
            if not supported:
                problems.add(_unsupported_fields("NamedTuple", expression))
            return problems
        if isinstance(expression, (ast.List, ast.Tuple)) and len(expression.elts) == 2:
            return self._scan_type(
                expression.elts[1], captures, scope, visiting, strings
            )
        if isinstance(expression, ast.IfExp):
            return self._scan_named_field(
                expression.body, captures, scope, visiting, strings
            ) | self._scan_named_field(
                expression.orelse, captures, scope, visiting, strings
            )
        return {_unsupported_fields("NamedTuple", expression)}

    def _scan_typed_fields(
        self,
        expression: ast.expr,
        captures: _Bindings,
        scope: _Scope,
        visiting: set[tuple[int, str]],
        strings: set[_ForwardStringKey],
    ) -> set[_Problem]:
        if isinstance(expression, ast.Name):
            key = (id(expression), "typed-dict-fields")
            if key in visiting:
                return set()
            visiting.add(key)
            alternatives, supported = self._expression_alternatives(
                expression, captures, scope
            )
            problems = set()
            for alternative, alternative_captures, alternative_scope in alternatives:
                problems.update(
                    self._scan_typed_fields(
                        alternative,
                        alternative_captures,
                        alternative_scope,
                        visiting,
                        strings,
                    )
                )
            visiting.remove(key)
            if not supported:
                problems.add(_unsupported_fields("TypedDict", expression))
            return problems
        if isinstance(expression, ast.Dict):
            problems = set()
            for key, value in zip(expression.keys, expression.values, strict=True):
                if key is None:
                    problems.update(
                        self._scan_typed_fields(
                            value, captures, scope, visiting, strings
                        )
                    )
                else:
                    problems.update(
                        self._scan_type(value, captures, scope, visiting, strings)
                    )
            return problems
        if isinstance(expression, ast.BinOp) and isinstance(expression.op, ast.BitOr):
            return self._scan_typed_fields(
                expression.left, captures, scope, visiting, strings
            ) | self._scan_typed_fields(
                expression.right, captures, scope, visiting, strings
            )
        if isinstance(expression, ast.IfExp):
            return self._scan_typed_fields(
                expression.body, captures, scope, visiting, strings
            ) | self._scan_typed_fields(
                expression.orelse, captures, scope, visiting, strings
            )
        if isinstance(expression, ast.Call):
            kinds = self._possible_kinds(expression.func, captures, scope, set())
            if not kinds or not kinds <= {"builtin-dict"}:
                return {_unsupported_fields("TypedDict", expression)}
            problems = set()
            for argument in expression.args:
                if isinstance(argument, (ast.List, ast.Tuple)):
                    problems.update(
                        self._scan_typed_dict_pairs(
                            argument, captures, scope, visiting, strings
                        )
                    )
                else:
                    problems.update(
                        self._scan_typed_fields(
                            argument, captures, scope, visiting, strings
                        )
                    )
            for keyword in expression.keywords:
                if keyword.arg is None:
                    problems.update(
                        self._scan_typed_fields(
                            keyword.value, captures, scope, visiting, strings
                        )
                    )
                else:
                    problems.update(
                        self._scan_type(
                            keyword.value, captures, scope, visiting, strings
                        )
                    )
            return problems
        return {_unsupported_fields("TypedDict", expression)}

    def _scan_typed_dict_pairs(
        self,
        expression: ast.List | ast.Tuple,
        captures: _Bindings,
        scope: _Scope,
        visiting: set[tuple[int, str]],
        strings: set[_ForwardStringKey],
    ) -> set[_Problem]:
        problems: set[_Problem] = set()
        for element in expression.elts:
            if isinstance(element, ast.Starred):
                problems.update(
                    self._scan_typed_fields(
                        element.value, captures, scope, visiting, strings
                    )
                )
            elif isinstance(element, (ast.List, ast.Tuple)) and len(element.elts) == 2:
                problems.update(
                    self._scan_type(
                        element.elts[1], captures, scope, visiting, strings
                    )
                )
            else:
                problems.add(_unsupported_fields("TypedDict", element))
        return problems


def _assignment_names(target: ast.expr) -> set[str]:
    if isinstance(target, ast.Name):
        return {target.id}
    if isinstance(target, (ast.List, ast.Tuple)):
        return {
            name
            for element in target.elts
            for name in _assignment_names(element)
        }
    if isinstance(target, ast.Starred):
        return _assignment_names(target.value)
    return set()


def _pep695_alias_name(statement: ast.stmt) -> str | None:
    if type(statement).__name__ != "TypeAlias":
        return None
    name = getattr(statement, "name", None)
    return name.id if isinstance(name, ast.Name) else None


def _pep695_roots(declaration: ast.AST) -> tuple[ast.expr, ...]:
    roots: list[ast.expr] = []
    if type(declaration).__name__ == "TypeAlias":
        value = getattr(declaration, "value", None)
        if isinstance(value, ast.expr):
            roots.append(value)
    for parameter in getattr(declaration, "type_params", ()):
        for field_name in ("bound", "default", "default_value"):
            value = getattr(parameter, field_name, None)
            if isinstance(value, ast.expr) and value not in roots:
                roots.append(value)
    return tuple(roots)


def _type_parameter_names(declaration: ast.AST) -> tuple[str, ...]:
    names: list[str] = []
    for parameter in getattr(declaration, "type_params", ()):
        name = getattr(parameter, "name", None)
        if isinstance(name, ast.Name):
            name = name.id
        if isinstance(name, str):
            names.append(name)
    return tuple(names)


def _is_static_method(declaration: ast.FunctionDef | ast.AsyncFunctionDef) -> bool:
    return any(
        (isinstance(decorator, ast.Name) and decorator.id == "staticmethod")
        or (
            isinstance(decorator, ast.Attribute)
            and decorator.attr == "staticmethod"
        )
        for decorator in declaration.decorator_list
    )


def _literal_truth(expression: ast.expr | None) -> bool | None:
    if expression is None:
        return None
    if isinstance(expression, ast.BoolOp):
        values = tuple(_literal_truth(value) for value in expression.values)
        if isinstance(expression.op, ast.And):
            if any(value is False for value in values):
                return False
            if all(value is True for value in values):
                return True
            return None
        if isinstance(expression.op, ast.Or):
            if any(value is True for value in values):
                return True
            if all(value is False for value in values):
                return False
            return None
    if isinstance(expression, ast.UnaryOp) and isinstance(expression.op, ast.Not):
        value = _literal_truth(expression.operand)
        return None if value is None else not value
    try:
        value = ast.literal_eval(expression)
    except (ValueError, TypeError, SyntaxError, MemoryError, RecursionError):
        return None
    return bool(value)


def _literal_integer(expression: ast.expr) -> int | None:
    try:
        value = ast.literal_eval(expression)
    except (ValueError, TypeError, SyntaxError, MemoryError, RecursionError):
        return None
    return value if isinstance(value, int) and not isinstance(value, bool) else None


def _pattern_names(pattern: ast.pattern) -> set[str]:
    names: set[str] = set()
    for node in ast.walk(pattern):
        for attribute in ("name", "rest"):
            name = getattr(node, attribute, None)
            if isinstance(name, str):
                names.add(name)
    return names


def _pattern_is_irrefutable(pattern: ast.pattern) -> bool:
    if isinstance(pattern, ast.MatchAs):
        return pattern.pattern is None or _pattern_is_irrefutable(pattern.pattern)
    if isinstance(pattern, ast.MatchOr):
        return any(_pattern_is_irrefutable(child) for child in pattern.patterns)
    return False


def _unsupported_fields(kind: str, expression: ast.expr) -> _Problem:
    return _Problem(
        f"has an unsupported dynamic {kind} field specification",
        expression.lineno,
    )


def _unsupported_invocation(expression: ast.expr) -> _Problem:
    return _Problem(
        "has an unsupported dynamic special typing invocation",
        expression.lineno,
    )


def _unsupported_class_keyword_unpack(expression: ast.expr) -> _Problem:
    return _Problem(
        "has an unsupported dynamic TypedDict class keyword unpack",
        expression.lineno,
    )


def stub_annotation_issues(tree: ast.Module) -> tuple[str, ...]:
    """Return deterministic issues for unsafe or unprovable public stub types."""

    return _Analyzer().analyze(tree)
