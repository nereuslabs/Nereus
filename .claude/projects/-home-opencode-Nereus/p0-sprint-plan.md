# P0 Sprint Plan: Adaptive Diagnostics + LLM Probing Assessment (Issue #7)

> **Цель спринта:** Заменить статичный stub roadmap и regex-оценщик на адаптивный diagnostic agent + LLM-based probing assessment.
>
> **Продолжительность:** 2 недели (10 рабочих дней)
>
> **Команда:** 1 разработчик

---

## 📋 Обзор изменений

| Компонент | Текущее состояние | Целевое состояние |
|---|---|---|
| Roadmapper | 3 жёстко закодированные темы | Адаптивный, на основе diagnostic test |
| Diagnostic | Отсутствует | 3–5 вопросов → выявление пробелов |
| Examiner (stub) | Regex на "good"/"partial" | LLM-based assessment с RAG контекстом |
| State schema | `RoadmapTopic(id, title, description)` | + `difficulty, prerequisites, estimated_hours` |

---

## 🏗️ Этап 1: Схемы и модели (День 1–2)

### 1.1. Расширить `RoadmapTopic` в `core/state.py`

```python
class RoadmapTopic(BaseModel):
    id: str
    title: str
    description: str
    difficulty: float = Field(default=1.0, ge=0.0, le=1.0)  # добавить
    prerequisites: list[str] = Field(default_factory=list)  # добавить
    estimated_hours: float = Field(default=1.0, ge=0.0)  # добавить
```

**Тесты:** `tests/unit/test_state.py` — добавить валидацию новых полей.

### 1.2. Добавить схемы для diagnostic

В `llm/schema.py`:

```python
class DiagnosticQuestion(BaseModel):
    id: str
    question: str
    options: list[str]


class DiagnosticOutput(BaseModel):
    questions: list[DiagnosticQuestion]


class WeaknessReport(BaseModel):
    weak_areas: list[str] = Field(default_factory=list)
    recommended_topics: list[str] = Field(default_factory=list)  # topic_ids
```

**Тесты:** `tests/unit/test_schema.py` — валидация новых схем.

### 1.3. Добавить роль DIAGNOSTIC в `llm/params.py`

```python
class AgentRole(str, Enum):
    COACH = "coach"
    TUTOR = "tutor"
    EXAMINER = "examiner"
    DIAGNOSTIC = "diagnostic"  # добавить
    SUMMARIZER = "summarizer"


# per-role params
AgentRole.DIAGNOSTIC: ModelParams(temperature=0.3, max_tokens=1024, json_mode=True)
```

---

## 🏗️ Этап 2: Diagnostic Agent (День 3–4)

### 2.1. Создать `agents/diagnostic.py`

```python
class DiagnosticAgent(BaseAgent):
    """Generates a diagnostic quiz and evaluates user answers to identify knowledge gaps."""

    def generate_questions(self, profile: UserProfile) -> list[DiagnosticQuestion]:
        """LLM-based or stub generation of 3-5 diagnostic questions."""
        if is_offline_inference(self._inference):
            return self._stub_questions(profile)
        result = self._inference.generate(
            build_diagnostic_prompt(profile),
            role=AgentRole.DIAGNOSTIC,
            output_model=DiagnosticOutput,
        )
        return result.questions

    def evaluate_answers(self, questions, answers) -> WeaknessReport:
        """LLM-based or stub evaluation of answers → weak areas + recommended topics."""
        ...

    def _stub_questions(self, profile) -> list[DiagnosticQuestion]:
        """Deterministic fallback — returns canned questions per skill."""
        ...
```

### 2.2. Prompt builder в `llm/prompts.py`

```python
def build_diagnostic_prompt(profile: UserProfile) -> list[dict[str, str]]:
    """System prompt: 'generate 3-5 diagnostic questions to identify gaps...'"""
    ...


def build_weakness_evaluation_prompt(
    profile: UserProfile,
    questions: list[DiagnosticQuestion],
    answers: dict[str, str],
) -> list[dict[str, str]]:
    """System prompt: 'evaluate answers, identify weak areas, recommend roadmap topics...'"""
    ...
```

### 2.3. Интеграция в `main.py` (CLI)

```python
def run_diagnostic(graph, profile) -> WeaknessReport:
    """Interactive diagnostic quiz before roadmap generation."""
    questions = graph._diagnostic_agent.generate_questions(profile)
    answers = {}
    for q in questions:
        print(f"\n{q.question}")
        for i, opt in enumerate(q.options, 1):
            print(f"  {i}. {opt}")
        choice = input("Your answer (number): ").strip()
        answers[q.id] = choice
    return graph._diagnostic_agent.evaluate_answers(questions, answers)
```

**Тесты:**
- `tests/unit/test_diagnostic_agent.py` — LLM path с `FakeLLMProvider`, stub path
- `tests/integration/test_diagnostic_flow.py` — full diagnostic → weak report

---

## 🏗️ Этап 3: Адаптивный Roadmapper (День 5–6)

### 3.1. Расширить `CoachAgent` в `agents/coach.py`

```python
class CoachAgent(BaseAgent):
    def build_roadmap(self, profile, *, weakness_report: WeaknessReport | None = None) -> Roadmap:
        """If weakness_report is provided, build adaptive roadmap targeting gaps."""
        if is_offline_inference(self._inference):
            if weakness_report:
                return self._adaptive_stub_roadmap(profile, weakness_report)
            return self._default_stub_roadmap(profile)

        messages = build_coach_prompt(session=session, weakness_report=weakness_report)
        result = self._inference.generate(
            messages, role=AgentRole.COACH, output_model=AdaptiveRoadmapOutput
        )
        return Roadmap(topics=[RoadmapTopic(**t) for t in result.topics])
```

### 3.2. Расширить схему `RoadmapOutput` → `AdaptiveRoadmapOutput`

```python
class AdaptiveRoadmapOutput(BaseModel):
    topics: list[RoadmapTopicOutput]  # теперь с difficulty, prerequisites


class RoadmapTopicOutput(BaseModel):
    id: str
    title: str
    description: str
    difficulty: float = 1.0
    prerequisites: list[str] = []
    estimated_hours: float = 1.0
```

### 3.3. Prompt builder обновление

```python
def build_coach_prompt(
    session=None,
    weakness_report: WeaknessReport | None = None,
) -> list[dict[str, str]]:
    """If weakness_report provided, instruct model to prioritize gap-filling topics."""
    user_msg = "Build a roadmap now."
    if weakness_report:
        weak_str = ", ".join(weakness_report.weak_areas)
        rec_str = ", ".join(weakness_report.recommended_topics)
        user_msg += f"\n\nDiagnostic identified weak areas: {weak_str}.\nRecommended topics: {rec_str}.\nPrioritize these areas."
    ...
```

**Тесты:**
- `tests/unit/test_coach_adaptive.py` — проверка, что weakness_report влияет на roadmap
- `tests/integration/test_adaptive_pipeline.py` — diagnostic → coach → tutor → examiner

---

## 🏗️ Этап 4: LLM Probing Assessment (День 7–8)

### 4.1. Обновить `ExaminerAgent` в `agents/examiner.py`

```python
class ExaminerAgent(BaseAgent):
    def assess(self, submission: str, state: NereusState) -> dict:
        """Always use LLMEvaluator if inference client available — even in nearline mode."""
        topic = state["roadmap"].topics[state["current_topic_index"]]
        context = {
            "task": state["task"],
            "topic": topic.title,
            "session": state.get("session"),
            "retrieved": state.get("retrieved_chunks") or [],
            "difficulty": topic.difficulty,  # использовать для calibration
            "prerequisites_met": self._check_prerequisites(topic, state),  # новое
        }

        # LLMEvaluator уже реализован — убираем is_offline guard
        score, feedback, weak_areas = self._evaluator(submission, context)

        verdict = Verdict.PASS if score >= self._passing_threshold(topic) else Verdict.RETRY
        ...

    def _passing_threshold(self, topic: RoadmapTopic) -> float:
        """Higher difficulty → higher passing threshold (70–85)."""
        base = 70.0
        return base + (topic.difficulty * 15.0)  # difficulty 0.0 → 70, 1.0 → 85

    def _check_prerequisites(self, topic, state) -> bool:
        """Check if all prerequisite topics are mastered."""
        mastered = {a.topic_id for a in state["session"].completed if a.verdict == Verdict.PASS}
        return all(prereq in mastered for prereq in topic.prerequisites)
```

### 4.2. Обновить StubLLMProvider для экзаменатора

В `llm/stub.py`, добавить режим экзаменатора:

```python
class StubLLMProvider(LLMProvider):
    def __init__(self, responder=None, *, examiner_responder=None):
        self._responder = responder or echo_responder
        self._examiner_responder = examiner_responder or self._default_examiner
    
    def _default_examiner(self, messages, **_):
        """Return a valid AssessmentOutput JSON based on submission quality."""
        # Извлекаем submission из messages[-1]["content"]
        # Heuristic: длинный ответ → выше score
        ...
```

**Тесты:**
- `tests/unit/test_examiner_llm.py` — LLM path с `FakeLLMProvider`
- `tests/unit/test_passing_threshold.py` — difficulty-based thresholds
- `tests/unit/test_prerequisites.py` — prerequisite checking

---

## 🏗️ Этап 5: Graph Integration (День 9–10)

### 5.1. Обновить `core/graph.py`

```python
class NereusGraph:
    def __init__(self, ..., diagnostic_agent=None, ...):
        self._diagnostic_agent = diagnostic_agent or DiagnosticAgent(inference=inference, provider=provider)
        ...
    
    def _build(self, checkpointer):
        builder = StateGraph(NereusState)
        
        # Diagnostic as optional first step (controlled by settings)
        if settings.run_diagnostic:
            builder.add_node("diagnostic", self._diagnostic_node)
            builder.set_entry_point("diagnostic")
            builder.add_edge("diagnostic", "coach")
        else:
            builder.set_entry_point("coach")
        
        builder.add_node("coach", self._coach_agent.run)  # coach теперь принимает weakness_report
        ...
    
    def _diagnostic_node(self, state):
        """Run diagnostic quiz → weak report → store in state."""
        questions = self._diagnostic_agent.generate_questions(state["user_profile"])
        # In interactive mode: interrupt for answers
        # In non-interactive mode: use default answers / skip
        answers = interrupt(questions) if self._interactive else self._default_answers(questions)
        report = self._diagnostic_agent.evaluate_answers(questions, answers)
        return {"weakness_report": report}
```

### 5.2. Обновить `core/state.py`

```python
class NereusState(TypedDict, total=False):
    ...
    diagnostic_questions: Optional[list[DiagnosticQuestion]]
    user_diagnostic_answers: Optional[dict[str, str]]
    weakness_report: Optional[WeaknessReport]
    ...
```

### 5.3. Обновить `core/factory.py`

```python
def build_nereus_graph(
    *,
    diagnostic_agent=None,
    run_diagnostic: bool | None = None,
    ...
) -> NereusGraph:
    ...
    run_diagnostic = run_diagnostic if run_diagnostic is not None else settings.run_diagnostic
    ...
```

### 5.4. Обновить `config/settings.py`

```python
class Settings(BaseSettings):
    ...
    run_diagnostic: bool = False  # включать diagnostic перед roadmap
    diagnostic_question_count: int = 5
    ...
```

### 5.5. Обновить `main.py`

```python
# CLI: добавить --no-diagnostic флаг
parser.add_argument("--no-diagnostic", action="store_true", help="Skip diagnostic quiz")
parser.add_argument("--diagnostic", action="store_true", help="Force run diagnostic")

# В main():
if not args.no_diagnostic and settings.run_diagnostic:
    # Run interactive diagnostic
    weak_report = run_diagnostic(graph, profile)
    initial_state = {"user_profile": profile, "weakness_report": weak_report, ...}
else:
    initial_state = {"user_profile": profile, ...}
```

### 5.6. Обновить `eval_chain.py`

```python
# Добавить флаг --diagnostic
parser.add_argument("--diagnostic", action="store_true", help="Run with diagnostic quiz")
parser.add_argument("--no-diagnostic", action="store_true", help="Skip diagnostic")
```

**Тесты:**
- `tests/integration/test_full_pipeline.py` — обновить для optional diagnostic
- `tests/integration/test_diagnostic_integration.py` — diagnostic → coach → tutor → examiner cycle

---

## 🧪 Этап 6: Тесты и CI (параллельно всему)

### 6.1. Unit тесты (минимум 15 новых)

| Файл | Тесты |
|---|---|
| `test_state.py` | validation для `difficulty`, `prerequisites`, `estimated_hours` |
| `test_schema.py` | `DiagnosticOutput`, `WeaknessReport`, `AdaptiveRoadmapOutput` |
| `test_diagnostic_agent.py` | LLM path (FakeLLMProvider), stub fallback |
| `test_coach_adaptive.py` | weakness_report влияет на roadmap структуру |
| `test_examiner_llm.py` | LLM evaluator вызывается (не regex guard) |
| `test_passing_threshold.py` | difficulty → threshold mapping |
| `test_prerequisites.py` | prerequisite checking logic |

### 6.2. Integration тесты (3 новых)

| Файл | Описание |
|---|---|
| `test_diagnostic_flow.py` | diagnostic quiz → weak report → assert structure |
| `test_adaptive_pipeline.py` | full cycle: diagnostic → adaptive roadmap → tutor → examiner |
| `test_prerequisite_routing.py` | topics с prerequisites блокируются до освоения |

### 6.3. Live тесты

- Обновить `test_live_openrouter.py` — добавить diagnostic + adaptive roadmapper проверку

---

## 📦 Доставка (Release)

### 7.1. Обновить документацию

- `README.md` — добавить секцию "Диагностический тест"
- `docs/wiki/Roadmap.md` — Step 7: Adaptive diagnostics
- `docs/wiki/Architecture.md` — обновить diagram с Diagnostic layer

### 7.2. Tag release

```bash
git tag v0.6.0-adaptive
git push origin v0.6.0-adaptive
```

---

## 📊 Метрики / Definition of Done

| Метрика | Требование |
|---|---|
| Unit tests | ≥ 95 passing (было 80) |
| Integration tests | +3 новых, все passing |
| Coverage | ≥ 85% на новых модулях |
| ruff check | ✅ No errors |
| ruff format | ✅ All files formatted |
| Offline mode | ✅ StubLLMProvider работает без сети |
| Live mode | ✅ NEREUS_RUN_LIVE=1 pass (gated) |
| Manual QA | ✅ CLI: `python main.py` → diagnostic → roadmap → tutor → examiner |

---

## ⚠️ Риски и mitigations

| Риск | Mitigation |
|---|---|
| Diagnostic добавляет latency (2–3 LLM вызова) | Stub-режим: cached questions; LLM-режим: optional (`run_diagnostic=False` по умолчанию) |
| LLM оценка может быть inconsistent | Stub fallback + pass threshold на difficulty |
| Prerequisites могут создать deadlock | Graph с циклами в LangGraph; использовать retry_count ограничение |
| Schema migration breaking existing sessions | `RoadmapTopic` поля имеют defaults — backward compatible |