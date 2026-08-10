"""Diagnostic Agent for adaptive skill assessment (Issue #7).

Generates a short diagnostic quiz (3-5 questions), collects user answers, and
produces a WeaknessReport identifying knowledge gaps. Used as an optional
pre-roadmap step in NereusGraph.
"""

from __future__ import annotations

import logging
from typing import Any

from nereus.agents.base import BaseAgent
from nereus.core.state import (
    DiagnosticQuestion,
    LearningStatus,
    NereusState,
    UserProfile,
    WeaknessReport,
)
from nereus.llm.inference import StructuredInferenceClient, is_offline_inference
from nereus.llm.params import AgentRole
from nereus.llm.prompts import build_diagnostic_prompt, build_weakness_prompt
from nereus.llm.schema import DiagnosticOutput, WeaknessReportOutput

logger = logging.getLogger("nereus.agents.diagnostic")

# Default number of diagnostic questions for stub mode
STUB_QUESTION_COUNT = 5


class DiagnosticAgent(BaseAgent):
    """Agent that runs a diagnostic quiz to identify knowledge gaps.

    Offline (StubLLMProvider / no inference client) uses deterministic stub
    questions; a real provider generates questions via the model with retries,
    raising LLMUnavailableError when it cannot (#44/#45).
    """

    def __init__(
        self,
        *,
        inference: StructuredInferenceClient | None = None,
        provider: Any = None,
        question_count: int = STUB_QUESTION_COUNT,
    ) -> None:
        self._provider = provider
        if inference is not None:
            self._inference = inference
        elif provider is not None:
            self._inference = StructuredInferenceClient(provider)
        else:
            self._inference = None
        self._question_count = question_count

    # ------------------------------------------------------------------ #
    # Stub (deterministic) fallback                                        #
    # ------------------------------------------------------------------ #
    def _stub_questions(self, profile: UserProfile) -> list[DiagnosticQuestion]:
        """Generate deterministic diagnostic questions for offline mode."""
        skill = profile.skill
        level = profile.current_level.value
        questions = [
            DiagnosticQuestion(
                id="q1",
                question=f"Что такое {skill}? (выберите лучшее определение)",
                options=[
                    f"{skill} — это язык программирования/инструмент",
                    "Это вид спорта",
                    "Это еда",
                    "Не знаю",
                ],
            ),
            DiagnosticQuestion(
                id="q2",
                question=f"Какой ваш текущий уровень в {skill}?",
                options=[
                    f"Я только начинаю ({level})",
                    "Продвинутый уровень",
                    "Эксперт",
                    "Не занимался раньше",
                ],
            ),
            DiagnosticQuestion(
                id="q3",
                question="Что вызывает ошибку в коде?",
                options=[
                    "Синтаксическая ошибка",
                    "Правильный код",
                    "Комментарий",
                    "Функция",
                ],
            ),
            DiagnosticQuestion(
                id="q4",
                question=f"Какой тип данных подходит для хранения текста в {skill}?",
                options=["string", "int", "float", "bool"],
            ),
            DiagnosticQuestion(
                id="q5",
                question=f"Что такое переменная в контексте {skill}?",
                options=[
                    "Именованная ячейка памяти для хранения данных",
                    "Функция",
                    "Класс",
                    "Модуль",
                ],
            ),
        ]
        return questions[: self._question_count]

    @staticmethod
    def _answer_text(question: DiagnosticQuestion, answer_idx: str) -> str:
        """Resolve an answer index (e.g. "3") to the option text."""
        if answer_idx.isdigit():
            idx = int(answer_idx) - 1
            if 0 <= idx < len(question.options):
                return question.options[idx]
        return ""

    @staticmethod
    def _stub_weakness(
        profile: UserProfile,
        questions: list[DiagnosticQuestion],
        answers: dict[str, str],
    ) -> WeaknessReport:
        """Deterministic weakness evaluation for offline mode."""
        skill = profile.skill.lower()
        weak_areas: list[str] = []

        q_by_id = {q.id: q for q in questions}

        # Evaluate Q1 (definition knowledge)
        q1 = q_by_id.get("q1")
        if q1:
            q1_text = DiagnosticAgent._answer_text(q1, answers.get("q1", "")).lower()
            if "спорт" in q1_text or "еда" in q1_text or "не знаю" in q1_text:
                weak_areas.extend([f"{skill} basics", "terminology"])

        # Evaluate Q3 (error recognition)
        q3 = q_by_id.get("q3")
        if q3:
            q3_text = DiagnosticAgent._answer_text(q3, answers.get("q3", "")).lower()
            if "правильный" in q3_text or "комментарий" in q3_text:
                weak_areas.append(f"{skill} debugging")

        # Evaluate Q4 (data types)
        q4 = q_by_id.get("q4")
        if q4:
            q4_text = DiagnosticAgent._answer_text(q4, answers.get("q4", "")).lower()
            if "int" in q4_text or "float" in q4_text or "bool" in q4_text:
                weak_areas.append(f"{skill} data types")

        # Evaluate Q5 (basic concepts)
        q5 = q_by_id.get("q5")
        if q5:
            q5_text = DiagnosticAgent._answer_text(q5, answers.get("q5", "")).lower()
            if "функция" in q5_text or "класс" in q5_text or "модуль" in q5_text:
                weak_areas.append(f"{skill} variables and memory")

        # Deduplicate while preserving order
        seen: set[str] = set()
        unique_weak = []
        for area in weak_areas:
            if area not in seen:
                seen.add(area)
                unique_weak.append(area)

        # Recommend topics based on weak areas
        recommended: list[str] = []
        if any("basics" in a for a in unique_weak):
            recommended.append("1")  # fundamentals topic
        if any("data types" in a for a in unique_weak):
            recommended.extend(["1", "2"])
        if any("debugging" in a for a in unique_weak):
            recommended.append("3")
        if any("variables" in a for a in unique_weak):
            recommended.append("1")

        # Deduplicate and return
        seen_rec: set[str] = set()
        unique_rec = []
        for topic in recommended:
            if topic not in seen_rec:
                seen_rec.add(topic)
                unique_rec.append(topic)

        return WeaknessReport(
            weak_areas=unique_weak or [f"{skill} fundamentals"],
            recommended_topics=unique_rec,
        )

    # ------------------------------------------------------------------ #
    # LLM-backed generation                                             #
    # ------------------------------------------------------------------ #
    def generate_questions(self, profile: UserProfile) -> list[DiagnosticQuestion]:
        """Generate diagnostic questions (LLM or stub fallback)."""
        if is_offline_inference(self._inference):
            return self._stub_questions(profile)

        messages = build_diagnostic_prompt(profile)
        result = self._inference.generate(
            messages, role=AgentRole.DIAGNOSTIC, output_model=DiagnosticOutput
        )
        questions = [
            DiagnosticQuestion(id=q.id, question=q.question, options=q.options)
            for q in result.questions  # type: ignore[attr-defined]
        ]
        if not questions:
            return self._stub_questions(profile)
        return questions

    def evaluate_answers(
        self,
        profile: UserProfile,
        questions: list[DiagnosticQuestion],
        answers: dict[str, str],
    ) -> WeaknessReport:
        """Evaluate answers and produce weakness report (LLM or stub)."""
        if is_offline_inference(self._inference):
            return self._stub_weakness(profile, questions, answers)

        messages = build_weakness_prompt(profile, questions, answers)
        result = self._inference.generate(
            messages, role=AgentRole.WEAKNESS, output_model=WeaknessReportOutput
        )
        return WeaknessReport(
            weak_areas=list(result.weak_areas),
            recommended_topics=list(result.recommended_topics),
        )

    # ------------------------------------------------------------------ #
    # Agent interface                                                    #
    # ------------------------------------------------------------------ #
    def run(self, state: NereusState) -> dict:
        """Generate diagnostic questions and store them in state.

        In interactive mode, this node is followed by an interrupt for user answers.
        In non-interactive mode, stub answers are used (for testing).
        """
        profile = state.get("user_profile")
        if profile is None:
            raise ValueError("DiagnosticAgent requires user_profile in state")

        questions = self.generate_questions(profile)

        result: dict = {
            "diagnostic_questions": questions,
            "user_diagnostic_answers": {},
            "status": LearningStatus.COACHING.value,
        }
        return result
