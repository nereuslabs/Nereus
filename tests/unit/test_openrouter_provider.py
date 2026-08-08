from __future__ import annotations

import json

import httpx
import pytest

from nereus.llm.openrouter import OpenRouterError, OpenRouterProvider

_strip = OpenRouterProvider._strip_thinking

# Build the open/close delimiters the implementation actually targets, using
# ord lists so no literal tokens ever appear in this source file.
#   *_OPEN  : 7-char tag that the regex "<think\\b[^>]*>" matches
#   *_CLOSE : 8-char tag that _strip_thinking matches literally
_OPEN = bytes([60, 116, 104, 105, 110, 107, 62]).decode("ascii")
_CLOSE = bytes([60, 47, 116, 104, 105, 110, 107, 62]).decode("ascii")


def _handler(response_payload: dict, status: int = 200):
    """Build a MockTransport handler returning a fixed chat-completion payload."""

    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(status, json=response_payload)

    return handler


def test_openrouter_provider_requires_api_key() -> None:
    with pytest.raises(ValueError, match="api_key"):
        OpenRouterProvider(api_key="")


def test_openrouter_provider_posts_native_completions() -> None:
    captured: dict = {}

    def handler(request: httpx.Request) -> httpx.Response:
        captured["url"] = str(request.url)
        captured["body"] = json.loads(request.content)
        captured["auth"] = request.headers.get("Authorization")
        captured["referer"] = request.headers.get("HTTP-Referer")
        return httpx.Response(
            200,
            json={
                "model": "openrouter/free",
                "choices": [{"message": {"content": "assistant reply"}}],
            },
        )

    provider = OpenRouterProvider(
        api_key="secret",
        model="openrouter/free",
        http_referer="https://example.com",
        client=httpx.Client(transport=httpx.MockTransport(handler)),
    )
    try:
        out = provider.complete([{"role": "user", "content": "hi"}], json_mode=True)
    finally:
        provider.close()

    assert out == "assistant reply"
    assert captured["url"].endswith("/api/v1/chat/completions")
    assert captured["auth"] == "Bearer secret"
    assert captured["referer"] == "https://example.com"
    # OpenAI-compatible body shape (not the legacy /api/chat + options.format)
    assert captured["body"]["model"] == "openrouter/free"
    assert captured["body"]["max_tokens"] == 4096
    assert captured["body"]["stream"] is False
    assert captured["body"]["response_format"] == {"type": "json_object"}
    assert provider.last_model == "openrouter/free"
    # single call traced
    assert len(provider.calls) == 1
    assert provider.calls[0]["status"] == 200


def test_openrouter_provider_no_json_mode_omits_response_format() -> None:
    resp = {"choices": [{"message": {"content": "plain"}}]}
    captured: dict = {}

    def handler(request: httpx.Request) -> httpx.Response:
        captured["body"] = json.loads(request.content)
        return httpx.Response(200, json=resp)

    provider = OpenRouterProvider(
        api_key="k", client=httpx.Client(transport=httpx.MockTransport(handler))
    )
    try:
        assert provider.complete([{"role": "user", "content": "hi"}]) == "plain"
    finally:
        provider.close()

    assert "response_format" not in captured["body"]


def test_strip_thinking_removes_block() -> None:
    content = "reasoning" + _OPEN + "hidden chain-of-thought" + _CLOSE + "answer"
    assert _strip(content) == "reasoninganswer"


def test_strip_thinking_plain_text_unchanged() -> None:
    assert _strip("no reasoning delimiters here") == "no reasoning delimiters here"


def test_openrouter_provider_strips_thinking_in_complete() -> None:
    content = "r" + _OPEN + "hidden" + _CLOSE + "a"
    resp = {"choices": [{"message": {"content": content}}]}
    provider = OpenRouterProvider(
        api_key="k", client=httpx.Client(transport=httpx.MockTransport(_handler(resp)))
    )
    try:
        out = provider.complete([{"role": "user", "content": "hi"}])
    finally:
        provider.close()
    assert out == "ra"


def test_openrouter_provider_raises_on_auth_error() -> None:
    provider = OpenRouterProvider(
        api_key="k",
        client=httpx.Client(
            transport=httpx.MockTransport(_handler({"error": {"message": "bad key"}}, status=401))
        ),
    )
    with pytest.raises(OpenRouterError) as exc_info:
        provider.complete([{"role": "user", "content": "hi"}])
    assert "401" in str(exc_info.value)
    provider.close()


def test_openrouter_provider_retries_on_429_then_succeeds(monkeypatch) -> None:
    calls = {"n": 0}
    sleep_mock = []

    def sleep(seconds):  # noqa: ANN001
        sleep_mock.append(seconds)

    monkeypatch.setattr("nereus.llm.openrouter.time.sleep", sleep)
    monkeypatch.setattr("nereus.llm.openrouter.time.perf_counter", lambda *a: 0.0)

    def handler(request: httpx.Request) -> httpx.Response:
        calls["n"] += 1
        if calls["n"] < 3:
            return httpx.Response(429, headers={"Retry-After": "0.01"})
        return httpx.Response(200, json={"choices": [{"message": {"content": "ok"}}]})

    provider = OpenRouterProvider(
        api_key="k", client=httpx.Client(transport=httpx.MockTransport(handler))
    )
    try:
        out = provider.complete([{"role": "user", "content": "hi"}])
    finally:
        provider.close()
    assert out == "ok"
    assert calls["n"] == 3  # two 429s then success
    assert len(sleep_mock) == 2
