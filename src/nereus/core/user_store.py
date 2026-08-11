"""User profile storage (P1 — multi-user profiles, Issues #8/#57).

``UserStore`` persists :class:`UserProfile` rows to SQLite so a deployment can
hold several independent users. Each user maps 1:1 to a profile + a set of
:class:`UserSession` files (see :mod:`nereus.core.session`).

When SQLite is unavailable (no driver, unwritable path, etc.) the store falls
back to an in-process dict so tests and offline runs never fail.
"""

from __future__ import annotations

import logging
import sqlite3
import uuid
from pathlib import Path
from typing import TYPE_CHECKING, Any

from nereus.core.state import UserProfile

if TYPE_CHECKING:
    pass

logger = logging.getLogger("nereus.user_store")


class UserStore:
    """Create / retrieve user profiles in a SQLite table.

    Schema::

        CREATE TABLE users (
            user_id  TEXT PRIMARY KEY,
            profile  TEXT NOT NULL,  -- JSON of UserProfile
            created  REAL NOT NULL
        );

    A process-wide *memory* fallback (dict) is used when the SQLite backend
    can't be opened, keeping the system offline-first.
    """

    def __init__(self, db_path: str | Path | None = None, *, schema: str = "users") -> None:
        self._schema = schema
        self._db_path = Path(db_path) if db_path else None
        self._mem: dict[str, UserProfile] = {}
        self._use_sqlite = False
        self._init_sqlite()

    # ------------------------------------------------------------------ #
    def _init_sqlite(self) -> None:
        if self._db_path is None:
            return
        try:
            self._db_path.parent.mkdir(parents=True, exist_ok=True)
            conn = sqlite3.connect(str(self._db_path))
            conn.execute(
                f"""
                CREATE TABLE IF NOT EXISTS {self._schema} (
                    user_id  TEXT PRIMARY KEY,
                    profile  TEXT NOT NULL,
                    created  REAL NOT NULL
                )
                """
            )
            conn.commit()
            conn.close()
            self._use_sqlite = True
            logger.debug("UserStore initialised at %s", self._db_path)
        except Exception as exc:  # noqa: BLE001  — any DB failure → memory fallback
            logger.warning("UserStore SQLite unavailable (%s); using memory fallback", exc)
            self._use_sqlite = False

    def _conn(self) -> sqlite3.Connection:
        assert self._db_path is not None
        return sqlite3.connect(str(self._db_path))

    # ------------------------------------------------------------------ #
    def create_user(self, profile: UserProfile) -> str:
        """Insert *profile* and return a new ``user_id`` (UUID4 hex)."""
        user_id = uuid.uuid4().hex
        if self._use_sqlite:
            try:
                conn = self._conn()
                conn.execute(
                    f"INSERT INTO {self._schema} (user_id, profile, created) VALUES (?, ?, ?)",
                    (user_id, profile.model_dump_json(), _now()),
                )
                conn.commit()
                conn.close()
                logger.info("Created user %s (sqlite)", user_id[:8])
                return user_id
            except Exception as exc:  # noqa: BLE001
                logger.warning("SQLite insert failed (%s); falling back to memory", exc)
        self._mem[user_id] = profile
        return user_id

    def get_user(self, user_id: str) -> UserProfile | None:
        """Fetch a profile by id, or ``None`` if not found."""
        if not user_id:
            return None
        if self._use_sqlite:
            try:
                conn = self._conn()
                row = conn.execute(
                    f"SELECT profile FROM {self._schema} WHERE user_id = ?", (user_id,)
                ).fetchone()
                conn.close()
                if row:
                    return UserProfile.model_validate_json(row[0])
            except Exception as exc:  # noqa: BLE001
                logger.warning("SQLite read failed (%s); falling back to memory", exc)
        return self._mem.get(user_id)

    def list_users(self) -> list[tuple[str, UserProfile]]:
        """Return ``(user_id, profile)`` pairs for all users."""
        if self._use_sqlite:
            try:
                conn = self._conn()
                rows = conn.execute(
                    f"SELECT user_id, profile FROM {self._schema} ORDER BY created"
                ).fetchall()
                conn.close()
                return [(row[0], UserProfile.model_validate_json(row[1])) for row in rows]
            except Exception as exc:  # noqa: BLE001
                logger.warning("SQLite list failed (%s); falling back to memory", exc)
        return list(self._mem.items())

    def delete_user(self, user_id: str) -> bool:
        """Delete a user. Returns True if a row was removed."""
        if not user_id:
            return False
        if self._use_sqlite:
            try:
                conn = self._conn()
                cur = conn.execute(f"DELETE FROM {self._schema} WHERE user_id = ?", (user_id,))
                conn.commit()
                conn.close()
                return cur.rowcount > 0
            except Exception as exc:  # noqa: BLE001
                logger.warning("SQLite delete failed (%s); falling back to memory", exc)
        return self._mem.pop(user_id, None) is not None


class UserStoreRedis:
    """Redis-backed :class:`UserProfile` store (P1, issue #8/#57).

    Each user is stored as a Redis hash at ``{namespace}:{user_id}`` with two
    fields:

    * ``profile`` — JSON of the :class:`UserProfile`
    * ``created``  — epoch seconds (float)

    Like :class:`UserStore`, it degrades to an in-process memory dict whenever
    the configured Redis instance is unreachable, so production deployments fail
    open instead of hard-erroring on a transient Redis blip.
    """

    def __init__(
        self,
        host: str = "localhost",
        port: int = 6379,
        *,
        db: int = 0,
        client: Any | None = None,
        namespace: str = "users",
    ) -> None:
        self._namespace = namespace
        self._mem: dict[str, UserProfile] = {}
        self._use_redis = False
        if client is not None:
            # Explicit client (mainly for tests) — assume it speaks the redis-py
            # hash + scan API with decode_responses=True semantics.
            self._client = client
            self._use_redis = bool(self._ping(client))
        else:
            try:
                import redis

                self._client = redis.Redis(
                    host=host,
                    port=port,
                    db=db,
                    decode_responses=True,
                    socket_connect_timeout=2.0,
                )
                self._ping(self._client)
                self._use_redis = True
                logger.debug("UserStoreRedis initialised at %s:%s", host, port)
            except Exception as exc:  # noqa: BLE001  — any failure → memory fallback
                logger.warning("UserStoreRedis unavailable (%s); using memory fallback", exc)
                self._client = None
                self._use_redis = False

    # ------------------------------------------------------------------ #
    @staticmethod
    def _ping(client: Any) -> bool:
        try:
            return bool(client.ping())
        except Exception:  # noqa: BLE001
            return False

    def _key(self, user_id: str) -> str:
        return f"{self._namespace}:{user_id}"

    def _row_to_profile(self, row: dict[str, str] | None) -> UserProfile | None:
        if not row:
            return None
        payload = row.get("profile")
        if not payload:
            return None
        try:
            return UserProfile.model_validate_json(payload)
        except Exception as exc:  # noqa: BLE001  — corrupt hash → treat as missing
            logger.warning("UserStoreRedis corrupt profile (%s); skipping", exc)
            return None

    # ------------------------------------------------------------------ #
    def create_user(self, profile: UserProfile) -> str:
        """Insert *profile* and return a new ``user_id`` (UUID4 hex)."""
        user_id = uuid.uuid4().hex
        if self._use_redis:
            try:
                self._client.hset(
                    self._key(user_id),
                    mapping={"profile": profile.model_dump_json(), "created": _now_str()},
                )
                logger.info("Created user %s (redis)", user_id[:8])
                return user_id
            except Exception as exc:  # noqa: BLE001
                logger.warning("Redis create failed (%s); falling back to memory", exc)
        self._mem[user_id] = profile
        return user_id

    def get_user(self, user_id: str) -> UserProfile | None:
        """Fetch a profile by id, or ``None`` if not found."""
        if not user_id:
            return None
        if self._use_redis:
            try:
                return self._row_to_profile(self._client.hgetall(self._key(user_id)))
            except Exception as exc:  # noqa: BLE001
                logger.warning("Redis read failed (%s); falling back to memory", exc)
        return self._mem.get(user_id)

    def list_users(self) -> list[tuple[str, UserProfile]]:
        """Return ``(user_id, profile)`` pairs for all users."""
        if self._use_redis:
            try:
                result: list[tuple[str, UserProfile]] = []
                cursor = 0
                pattern = f"{self._namespace}:*"
                while True:
                    cursor, keys = self._client.scan(cursor=cursor, match=pattern, count=1000)
                    for key in keys:
                        profile = self._row_to_profile(self._client.hgetall(key))
                        if profile is not None:
                            uid = key.split(":", 1)[1]
                            result.append((uid, profile))
                    if cursor == 0:
                        break
                return result
            except Exception as exc:  # noqa: BLE001
                logger.warning("Redis list failed (%s); falling back to memory", exc)
        return list(self._mem.items())

    def delete_user(self, user_id: str) -> bool:
        """Delete a user. Returns True if a row was removed."""
        if not user_id:
            return False
        if self._use_redis:
            try:
                return self._client.delete(self._key(user_id)) > 0
            except Exception as exc:  # noqa: BLE001
                logger.warning("Redis delete failed (%s); falling back to memory", exc)
        return self._mem.pop(user_id, None) is not None


def _now_str() -> str:
    # Redis hashes store strings; keep `created` as a compact epoch string.
    import time

    return str(time.time())


def build_user_store(
    backend: str | None = None,
    *,
    db_path: str | Path | None = None,
    host: str = "localhost",
    port: int = 6379,
) -> UserStore | UserStoreRedis:
    """Construct the right ``UserStore`` variant from ``settings.user_storage``."""
    from nereus.config.settings import settings

    resolved = backend or settings.user_storage
    if resolved == "memory":
        return UserStore()  # db_path=None -> in-process dict backing
    if resolved == "redis":
        return UserStoreRedis(host=host, port=port)
    return UserStore(db_path=db_path or settings.user_db_path)


def _now() -> float:
    import time

    return time.time()
