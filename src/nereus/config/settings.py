from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_file=".env", env_file_encoding="utf-8", extra="ignore")

    env: str = "development"
    log_level: str = "INFO"

    # LLM provider: "stub" (in-memory, default) or "openrouter"
    llm_provider: str = "stub"

    # --- OpenRouter (cloud LLM + embeddings; replaces legacy Ollama) ---
    # OpenRouter is a unified, OpenAI-compatible facade over 100+ providers.
    # Use ``openrouter/free`` as the chat model to route to free tiers without
    # running any local model (no 8GB Ollama container). Requires OPENROUTER_API_KEY.
    openrouter_api_key: str = ""
    openrouter_base_url: str = "https://openrouter.ai/api/v1"
    openrouter_model: str = "openrouter/free"
    openrouter_timeout: float = 60.0
    # Optional leaderboard attribution headers (OpenRouter docs).
    openrouter_http_referer: str = ""
    openrouter_title: str = "Nereus"
    # Embed model used when EMBEDDING_PROVIDER=openrouter (cheap pay-as-you-go).
    openrouter_embed_model: str = "openai/text-embedding-3-small"

    chromadb_host: str = "localhost"
    chromadb_port: int = 8000

    # Approximate max tokens for the in-memory message history window.
    context_max_tokens: int = 8000

    # --- RAG / embeddings (Step 4) ---
    embedding_provider: str = "stub"  # "stub" | "sentence_transformers" | "openrouter"
    sentence_transformers_model: str = "sentence-transformers/all-MiniLM-L6-v2"
    retriever_top_k: int = 5

    # --- Persistence / checkpointer (Step 4+, issue #16) ---
    checkpoint_backend: str = "memory"  # "memory" | "sqlite" | "redis"
    checkpoint_db: str = ".checkpoints/nereus.sqlite3"
    redis_host: str = "localhost"
    redis_port: int = 6379
    checkpoint_ttl_seconds: int = 0  # 0 = no TTL

    # --- LearningSession dump/load (issue #22 runtime wiring) ---
    session_path: str = ".sessions/{thread_id}.json"  # pattern, {thread_id} substituted

    @property
    def redis_url(self) -> str:
        return f"redis://{self.redis_host}:{self.redis_port}/0"

    def chromadb_url(self) -> str:
        return f"http://{self.chromadb_host}:{self.chromadb_port}"


settings = Settings()
