#!/usr/bin/env python3
"""Delta-debug one dumped OpenRouter request against a provider failure."""

from __future__ import annotations

import argparse
import copy
import json
import math
import os
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Callable, Sequence


Json = dict[str, Any] | list[Any] | str | int | float | bool | None


class ProbeBudgetExhausted(RuntimeError):
    """The configured live-call or successful-response cost budget was spent."""


class ProbeClient:
    def __init__(
        self,
        *,
        api_key: str,
        url: str,
        failure_pattern: re.Pattern[str],
        failure_status: int,
        max_calls: int,
        max_success_cost: float,
        timeout: float,
    ) -> None:
        self.api_key = api_key
        self.url = url
        self.failure_pattern = failure_pattern
        self.failure_status = failure_status
        self.max_calls = max_calls
        self.max_success_cost = max_success_cost
        self.timeout = timeout
        self.calls = 0
        self.success_cost = 0.0

    def probe(self, body: bytes, label: str) -> bool:
        if self.calls >= self.max_calls:
            raise ProbeBudgetExhausted(
                f"live-call budget exhausted after {self.calls} calls"
            )
        if self.success_cost >= self.max_success_cost:
            raise ProbeBudgetExhausted(
                "successful-response cost budget exhausted at "
                f"${self.success_cost:.6f}"
            )

        self.calls += 1
        request = urllib.request.Request(
            self.url,
            data=body,
            headers={
                "Authorization": f"Bearer {self.api_key}",
                "Content-Type": "application/json",
            },
            method="POST",
        )
        try:
            with urllib.request.urlopen(request, timeout=self.timeout) as response:
                status = response.status
                response_body = response.read()
        except urllib.error.HTTPError as error:
            status = error.code
            response_body = error.read()
        except urllib.error.URLError as error:
            print(
                f"[{self.calls:03}] KEEP  {label}: transport error: {error}",
                file=sys.stderr,
                flush=True,
            )
            return False

        text = response_body.decode("utf-8", errors="replace")
        if 200 <= status <= 299:
            self.success_cost += response_cost(response_body)
        matches = status == self.failure_status and self.failure_pattern.search(text) is not None
        verdict = "FAIL" if matches else "KEEP"
        print(
            f"[{self.calls:03}] {verdict:<5} {label}: HTTP {status}; "
            f"success cost ${self.success_cost:.6f}",
            file=sys.stderr,
            flush=True,
        )
        return matches


def response_cost(response_body: bytes) -> float:
    try:
        response = json.loads(response_body)
        cost = response.get("usage", {}).get("cost", 0.0)
        return float(cost) if isinstance(cost, (int, float)) and cost >= 0 else 0.0
    except (json.JSONDecodeError, TypeError, ValueError):
        return 0.0


def compact_json(value: Json) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def get_at(value: Json, path: tuple[str | int, ...]) -> Json:
    current = value
    for component in path:
        current = current[component]  # type: ignore[index]
    return current


def replace_at(value: Json, path: tuple[str | int, ...], replacement: Json) -> Json:
    if not path:
        return replacement
    candidate = copy.deepcopy(value)
    parent = get_at(candidate, path[:-1])
    parent[path[-1]] = replacement  # type: ignore[index]
    return candidate


def path_label(path: tuple[str | int, ...]) -> str:
    if not path:
        return "$"
    return "$" + "".join(
        f"[{component}]" if isinstance(component, int) else f".{component}"
        for component in path
    )


class RequestReducer:
    def __init__(
        self,
        value: Json,
        client: ProbeClient,
        *,
        minimize_strings: bool,
    ) -> None:
        self.value = value
        self.client = client
        self.minimize_strings = minimize_strings

    def try_replacement(
        self,
        path: tuple[str | int, ...],
        replacement: Json,
        label: str,
    ) -> bool:
        candidate = replace_at(self.value, path, replacement)
        if compact_json(candidate) == compact_json(self.value):
            return False
        if self.client.probe(compact_json(candidate), label):
            self.value = candidate
            return True
        return False

    def ddmin_items(
        self,
        path: tuple[str | int, ...],
        items: Sequence[Any],
        build: Callable[[Sequence[Any]], Json],
        *,
        minimum_size: int,
        label: str,
    ) -> list[Any]:
        current = list(items)
        granularity = 2
        while len(current) > minimum_size:
            chunk_size = math.ceil(len(current) / granularity)
            reduced = False
            for start in range(0, len(current), chunk_size):
                candidate = current[:start] + current[start + chunk_size :]
                if len(candidate) < minimum_size:
                    continue
                removed = len(current) - len(candidate)
                if self.try_replacement(
                    path,
                    build(candidate),
                    f"{label}: remove {removed}/{len(current)}",
                ):
                    current = candidate
                    granularity = max(2, granularity - 1)
                    reduced = True
                    break
            if reduced:
                continue
            if granularity >= len(current):
                break
            granularity = min(len(current), granularity * 2)
        return current

    def minimize_mapping(
        self,
        path: tuple[str | int, ...],
        preserved_keys: set[str],
    ) -> None:
        mapping = get_at(self.value, path)
        assert isinstance(mapping, dict)
        removable = [key for key in mapping if key not in preserved_keys]

        def build(kept_removable: Sequence[str]) -> Json:
            kept = preserved_keys | set(kept_removable)
            current = get_at(self.value, path)
            assert isinstance(current, dict)
            return {key: item for key, item in current.items() if key in kept}

        self.ddmin_items(
            path,
            removable,
            build,
            minimum_size=0,
            label=f"{path_label(path)} fields",
        )

    def minimize_list(
        self,
        path: tuple[str | int, ...],
        *,
        minimum_size: int,
    ) -> None:
        items = get_at(self.value, path)
        assert isinstance(items, list)
        self.ddmin_items(
            path,
            items,
            lambda candidate: list(candidate),
            minimum_size=minimum_size,
            label=f"{path_label(path)} elements",
        )

    def minimize_string(self, path: tuple[str | int, ...]) -> None:
        text = get_at(self.value, path)
        assert isinstance(text, str)
        self.ddmin_items(
            path,
            list(text),
            lambda characters: "".join(characters),
            minimum_size=0,
            label=f"{path_label(path)} characters",
        )

    def minimize_value(
        self,
        path: tuple[str | int, ...],
        *,
        minimize_container: bool = True,
    ) -> None:
        value = get_at(self.value, path)
        if isinstance(value, dict):
            if minimize_container:
                self.minimize_mapping(path, set())
            current = get_at(self.value, path)
            assert isinstance(current, dict)
            for key in list(current):
                self.minimize_value(path + (key,))
        elif isinstance(value, list):
            if minimize_container:
                self.minimize_list(path, minimum_size=0)
            current = get_at(self.value, path)
            assert isinstance(current, list)
            for index in range(len(current)):
                self.minimize_value(path + (index,))
        elif (
            self.minimize_strings
            and isinstance(value, str)
            and path != ("model",)
        ):
            self.minimize_string(path)

    def run(self) -> Json:
        root = self.value
        if not isinstance(root, dict):
            raise ValueError("the dumped request must be a JSON object")

        # Keep enough routing to continue exercising Xiaomi. Nested provider
        # fields are still minimized and must prove that they are necessary.
        self.minimize_mapping((), {"model", "provider", "messages"})
        current = self.value
        assert isinstance(current, dict)
        if isinstance(current.get("messages"), list):
            self.minimize_list(("messages",), minimum_size=1)
        current = self.value
        assert isinstance(current, dict)
        if isinstance(current.get("tools"), list):
            self.minimize_list(("tools",), minimum_size=0)

        current = self.value
        assert isinstance(current, dict)
        for key in list(current):
            if key == "model":
                continue
            self.minimize_value(
                (key,),
                minimize_container=key not in {"messages", "tools"},
            )
        return self.value


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("request", type=Path, help="numbered *-request.json dump")
    parser.add_argument("--output", type=Path, required=True, help="minimal request JSON")
    parser.add_argument(
        "--url",
        default="https://openrouter.ai/api/v1/chat/completions",
        help="OpenRouter-compatible completion URL",
    )
    parser.add_argument(
        "--match",
        default="unexpected end of data",
        help="regular expression required in the failing response body",
    )
    parser.add_argument("--status", type=int, default=400, help="failing HTTP status")
    parser.add_argument("--max-calls", type=int, default=48)
    parser.add_argument("--max-success-cost-usd", type=float, default=1.0)
    parser.add_argument("--timeout-seconds", type=float, default=120.0)
    parser.add_argument(
        "--minimize-strings",
        action="store_true",
        help="also delta-debug string contents after structural minimization",
    )
    args = parser.parse_args()
    if args.max_calls <= 0:
        parser.error("--max-calls must be positive")
    if args.max_success_cost_usd <= 0:
        parser.error("--max-success-cost-usd must be positive")
    if args.timeout_seconds <= 0:
        parser.error("--timeout-seconds must be positive")
    if not 100 <= args.status <= 599:
        parser.error("--status must be between 100 and 599")
    return args


def main() -> int:
    args = parse_args()
    api_key = os.environ.get("OPENROUTER_API_KEY", "").strip()
    if not api_key:
        print("OPENROUTER_API_KEY is not set", file=sys.stderr)
        return 2

    raw_request = args.request.read_bytes()
    try:
        request_value = json.loads(raw_request)
    except json.JSONDecodeError as error:
        print(f"invalid request JSON: {error}", file=sys.stderr)
        return 2
    if not isinstance(request_value, dict):
        print("the dumped request must be a JSON object", file=sys.stderr)
        return 2

    client = ProbeClient(
        api_key=api_key,
        url=args.url,
        failure_pattern=re.compile(args.match),
        failure_status=args.status,
        max_calls=args.max_calls,
        max_success_cost=args.max_success_cost_usd,
        timeout=args.timeout_seconds,
    )
    if not client.probe(raw_request, "exact dumped baseline"):
        print("the exact dumped body did not reproduce the selected failure", file=sys.stderr)
        return 1

    reducer = RequestReducer(
        request_value,
        client,
        minimize_strings=args.minimize_strings,
    )
    try:
        minimal = reducer.run()
    except ProbeBudgetExhausted as error:
        print(f"budget stop: {error}", file=sys.stderr)
        minimal = reducer.value

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(json.dumps(minimal, ensure_ascii=False, indent=2).encode("utf-8") + b"\n")
    print(
        f"wrote {args.output}; calls={client.calls}; "
        f"successful-response-cost=${client.success_cost:.6f}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
