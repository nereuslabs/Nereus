"""CLI driver for the Nereus MVP learning automaton.

Runs the LangGraph pipeline in human-in-the-loop mode: the graph pauses at
the examiner node, the user types their answer, and the run resumes until the
whole roadmap is completed.
"""

from __future__ import annotations

from langgraph.checkpoint.memory import MemorySaver
from langgraph.types import Command

from nereus.core.graph import NereusGraph
from nereus.core.state import UserLevel, UserProfile

INTERRUPT_KEY = "__interrupt__"


def build_profile() -> UserProfile:
    print("=== Nereus Coach: let's build your learning profile ===")
    skill = input("Skill to learn [Python]: ").strip() or "Python"
    current = input("Current level (beginner/intermediate/advanced) [beginner]: ").strip()
    target = input("Target level (beginner/intermediate/advanced) [intermediate]: ").strip()
    hours = float(input("Hours per day you can study [1]: ").strip() or "1")
    deadline = int(input("In how many days do you want to finish [30]: ").strip() or "30")

    return UserProfile(
        skill=skill,
        current_level=UserLevel(current or "beginner"),
        target_level=UserLevel(target or "intermediate"),
        hours_per_day=hours,
        deadline_days=deadline,
        goal=f"Master {skill}",
    )


def main() -> None:
    profile = build_profile()
    graph = NereusGraph(checkpointer=MemorySaver(), interactive=True)
    config = {"configurable": {"thread_id": "nereus-demo"}}

    final = graph.invoke({"user_profile": profile}, config)

    while INTERRUPT_KEY in final and final[INTERRUPT_KEY]:
        interrupt = final[INTERRUPT_KEY][0]
        task = interrupt.value["task"]
        print(f"\n[ Tutor ] {final['material']}")
        print(f"[ Examiner ] {task}")
        answer = input("Your answer (type\"good\" to pass): ").strip()
        final = graph.invoke(Command(resume=answer), config)

    print("\n=== Roadmap completed! ===")
    for topic in final["roadmap"].topics:
        print(f"- {topic.title}")


if __name__ == "__main__":
    raise SystemExit(main())