from __future__ import annotations

import json

import pytest

from nereus.llm.ollama import OllamaProvider
from nereus.llm.schema import extract_json
from nereus.llm.stub import StubLLMProvider


def test_extract_json_plain() -> None:
    assert extract_json('{"a": 1}') == {"a": 1}


def test_extract_json_fenced() -> None:
    text = 'Here you go:\n```json\n{"topics": []}\n```\nHope it helps.'
    assert extract_json(text) == {"topics": []}


def test_extract_json_with_prose_prefix() -> None:
    text = 'The roadmap is: {"score": 90, "feedback": "ok"}'
    assert extract_json(text)["score"] == 90


def test_extract_json_raises_on_garbage() -> None:
    with pytest.raises(ValueError):
        extract_json("no json here at all")


def test_stub_provider_echoes_last_user_message() -> None:
    provider = StubLLMProvider()
    out = provider.complete(
        [{"role": "user", "content": "hello"}]
    )
    assert out == "hello"
    assert len(provider.calls) == 1


def test_stub_provider_custom_responder() -> None:
    def responder(messages, **_):
        return json.dumps({"ok": True})

    provider = StubLLMProvider(responder=responder)
    assert json.loads(provider.complete([])) == {"ok": True}


def test_ollama_provider_posts_native_chat() -> None:
    import httpx

    captured: dict = {}

    def handler(request: httpx.Request) -> httpx.Response:
        captured["url"] = str(request.url)
        captured["body"] = json.loads(request.content)
        captured["auth"] = request.headers.get("Authorization")
        return httpx.Response(200, json={"message": {"content": "assistant reply"}})

    transport = httpx.MockTransport(handler)
    provider = OllamaProvider(
        base_url="http://ollama:11434",
        model="gemma4:31b-cloud",
        api_key="secret",
        client=httpx.Client(transport=transport),
    )
    try:
        out = provider.complete([{"role": "user", "content": "hi"}], json_mode=True)
    finally:
        provider.close()

    assert out == "assistant reply"
    assert captured["url"].endswith("/api/chat")
    assert captured["body"]["model"] == "gemma4:31b-cloud"
    assert captured["body"]["format"] == "json"
    assert captured["body"]["stream"] is False
    assert captured["auth"] == "Bearer secret"


def test_system_prompts_mandate_russian_output() -> None:
    """#52 regression: every agent system prompt must carry the Russian language
    directive so the OpenRouter model stops switching to English."""
    from nereus.llm.params import AgentRole
    from nereus.llm.prompts import LANGUAGE_INSTRUCTION, _system

    assert "русск" in LANGUAGE_INSTRUCTION.lower()
    for role in AgentRole:
        sys_prompt = _system(role)
        assert LANGUAGE_INSTRUCTION in sys_prompt, role
        assert "на русском" in sys_prompt.lower(), role