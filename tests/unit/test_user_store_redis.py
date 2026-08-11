"""Unit tests for the Redis user store backend (P1, issue #8/#57).

Uses an in-process dict-based fake that mimics the redis-py hash + scan API
(``decode_responses=True`` semantics), so the suite stays fully hermetic — no
real Redis is required.
"""

from __future__ import annotations

import fnmatch

import pytest

from nereus.core.state import UserLevel, UserProfile
from nereus.core.user_store import UserStoreRedis


def _profile(skill: str = "Python") -> UserProfile:
    return UserProfile(
        skill=skill,
        current_level=UserLevel.BEGINNER,
        target_level=UserLevel.INTERMEDIATE,
        hours_per_day=1.0,
        deadline_days=30,
        goal="Learn",
    )


class _FakeRedis:
    """Minimal redis-py-compatible client (hash + scan + ping/del)."""

    def __init__(self) -> None:
        self._h: dict[str, dict[str, str]] = {}

    def ping(self) -> bool:
        return True

    def hset(self, name, mapping=None, **kw) -> int:
        m = {**(mapping or {}), **kw}
        self._h.setdefault(name, {}).update(m)
        return len(m)

    def hgetall(self, name) -> dict[str, str]:
        return dict(self._h.get(name, {}))

    def delete(self, *names) -> int:
        removed = 0
        for name in names:
            if name in self._h:
                del self._h[name]
                removed += 1
        return removed

    def scan(self, cursor=0, match=None, count=None):
        keys = list(self._h.keys())
        if match:
            keys = [k for k in keys if fnmatch.fnmatch(k, match)]
        # Return everything in one shot, then signal completion.
        return (0, keys)


@pytest.fixture
def fake_redis() -> _FakeRedis:
    return _FakeRedis()


def test_redis_roundtrip_create_get_delete(fake_redis) -> None:
    store = UserStoreRedis(client=fake_redis, namespace="users")
    uid = store.create_user(_profile("Python"))
    assert uid  # non-empty UUID hex

    got = store.get_user(uid)
    assert got is not None
    assert got.skill == "Python"

    listed = store.list_users()
    assert len(listed) == 1
    assert listed[0][0] == uid
    assert listed[0][1].skill == "Python"

    assert store.delete_user(uid) is True
    assert store.get_user(uid) is None
    assert store.list_users() == []


def test_redis_get_missing_returns_none(fake_redis) -> None:
    store = UserStoreRedis(client=fake_redis)
    assert store.get_user("nope") is None
    assert store.delete_user("nope") is False


def test_redis_corrupt_profile_is_skipped(fake_redis) -> None:
    store = UserStoreRedis(client=fake_redis)
    uid = store.create_user(_profile("Python"))
    # Poison the stored JSON on the fake backend.
    fake_redis.hset(f"users:{uid}", mapping={"profile": "{not-json", "created": "0"})

    assert store.get_user(uid) is None  # corrupt -> treated as missing
    # But the (corrupted) key still exists for list; non-corrupt users still listable.
    assert uid not in [u for u, _ in store.list_users()]


@pytest.mark.parametrize("unreachable", [True, False])
def test_user_store_backend_switch(fake_redis, unreachable) -> None:
    """Both backends expose identical CRUD semantics."""
    if unreachable:
        store = UserStoreRedis(client=_UnreachableRedis())
    else:
        store = UserStoreRedis(client=fake_redis)
    uid = store.create_user(_profile("Python"))
    assert store.get_user(uid).skill == "Python"
    assert len(store.list_users()) == 1
    assert store.delete_user(uid) is True


class _UnreachableRedis:
    def ping(self) -> bool:
        raise ConnectionError("boom")  # noqa: TRY301 — simulate a network blip


def test_redis_unavailable_falls_back_to_memory() -> None:
    """When Redis is unreachable the store must still work via memory fallback."""
    store = UserStoreRedis(client=_UnreachableRedis())
    assert store._use_redis is False  # noqa: SLF001

    uid = store.create_user(_profile("Python"))
    assert store.get_user(uid).skill == "Python"
    assert len(store.list_users()) == 1
    assert store.delete_user(uid) is True
    assert store.get_user(uid) is None


def test_build_user_store_dispatch(monkeypatch) -> None:
    import os
    import tempfile

    from nereus.core.user_store import UserStore, UserStoreRedis, build_user_store

    # memory backend -> in-process UserStore (no DB file)
    assert isinstance(build_user_store("memory"), UserStore)

    # redis backend -> UserStoreRedis; stub __init__ so no real connection is made
    monkeypatch.setattr(UserStoreRedis, "__init__", lambda self, *a, **k: None)
    assert isinstance(build_user_store("redis"), UserStoreRedis)

    # sqlite backend -> UserStore backed by a real tmp DB
    with tempfile.TemporaryDirectory() as d:
        s = build_user_store("sqlite", db_path=os.path.join(d, "u.sqlite3"))
        assert isinstance(s, UserStore)
        uid = s.create_user(_profile("Python"))
        assert s.get_user(uid) is not None
