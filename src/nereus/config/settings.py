from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_file=".env", env_file_encoding="utf-8", extra="ignore")

    env: str = "development"
    log_level: str = "INFO"

    # LLM provider: "stub" (in-memory, default) or "ollama"
    llm_provider: str = "stub"

    ollama_base_url: str = "http://localhost:11434"
    ollama_model: str = "gemma4:31b"
    ollama_api_key: str = ""
    ollama_timeout: float = 120.0

    # Ollama Cloud for chat; local Ollama for cheap stateless embeddings.
    # Leave empty -> falls back to ollama_base_url.
    ollama_embed_base_url: str = ""

    chromadb_host: str = "localhost"
    chromadb_port: int = 8000

    # Approximate max tokens for the in-memory message history window.
    context_max_tokens: int = 8000

    # --- RAG / embeddings (Step 4) ---
    embedding_provider: str = "stub"  # "stub" | "sentence_transformers" | "ollama"
    sentence_transformers_model: str = "sentence-transformers/all-MiniLM-L6-v2"
    ollama_embed_model: str = "nomic-embed-text"
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
