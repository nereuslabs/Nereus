"""CLI driver for the Nereus MVP learning automaton.

Runs the LangGraph pipeline in human-in-the-loop mode: the graph pauses at
the examiner node, the user types their answer, and the run resumes until the
whole roadmap is completed.

Use ``--resume <thread_id>`` to resume a previously saved learning session;
the checkpoint is loaded from ``settings.checkpoint_db`` (SQLite by default).
"""

from __future__ import annotations

import argparse
import logging

from langgraph.types import Command

from nereus.core.factory import build_nereus_graph
from nereus.core.state import UserLevel, UserProfile

INTERRUPT_KEY = "__interrupt__"


def build_profile() -> UserProfile:
    print("=== Nereus Coach: let's build your learning profile ===")
    skill = input("Skill to learn [Python]: ").strip() or "Python"
    current = input(
        "Current level (beginner/intermediate/advanced) [beginner]: "
    ).strip()
    target = input(
        "Target level (beginner/intermediate/advanced) [intermediate]: "
    ).strip()
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


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="nereus",
        description=__doc__,
    )
    parser.add_argument(
        "--resume",
        metavar="THREAD_ID",
        help="Resume a saved session by thread_id (restores profile/roadmap/progress)",
    )
    parser.add_argument(
        "--checkpoint-backend",
        choices=["memory", "sqlite", "redis"],
        help="Override CHECKPOINT_BACKEND env setting",
    )
    args = parser.parse_args(argv)

    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(levelname)s %(name)s | %(message)s",
    )
    logger = logging.getLogger("nereus")

    if args.resume:
        config = {"configurable": {"thread_id": args.resume}}
        # no profile prompt; load existing state from checkpoint
        initial_state: dict = {}
    else:
        profile = build_profile()
        logger.info(
            "starting run | skill=%r goal=%r target=%s",
            profile.skill,
            profile.goal,
            profile.target_level,
        )
        graph = build_nereus_graph(
            interactive=True, checkpoint=args.checkpoint_backend
        )
        config = {"configurable": {"thread_id": "nereus-demo"}}
        initial_state = {"user_profile": profile}

    graph = build_nereus_graph(
        interactive=True, checkpoint=args.checkpoint_backend
    )

    final = graph.invoke(initial_state, config)

    while INTERRUPT_KEY in final and final[INTERRUPT_KEY]:
        interrupt = final[INTERRUPT_KEY][0]
        task = interrupt.value["task"]
        print(f"\n[ Tutor ] {final.get('material', '')}")
        print(f"[ Examiner ] {task}")
        answer = input('Your answer (type "good" to pass): ').strip()
        final = graph.invoke(Command(resume=answer), config)

    print("\n=== Roadmap completed! ===")
    for topic in final.get("roadmap", {}).get("topics", []):
        print(f"- {topic.title}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())