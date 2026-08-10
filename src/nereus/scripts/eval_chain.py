from __future__ import annotations

import argparse
import json
import logging
from pathlib import Path

from nereus.core.factory import build_nereus_graph
from nereus.core.session import LearningSession
from nereus.core.state import UserLevel, UserProfile, WeaknessReport

logger = logging.getLogger("nereus")


def build_profile(args: argparse.Namespace) -> UserProfile:
    return UserProfile(
        skill=args.skill,
        current_level=UserLevel(args.current_level),
        target_level=UserLevel(args.target_level),
        hours_per_day=args.hours,
        deadline_days=args.deadline,
        goal=args.goal or f"Master {args.skill}",
    )


def run(args: argparse.Namespace) -> dict:
    profile = build_profile(args)
    graph = build_nereus_graph(interactive=False, run_diagnostic=args.diagnostic)

    initial_state: dict = {
        "user_profile": profile,
        "user_submission": args.submission,
        "max_retries": args.max_retries,
    }

    # If diagnostic is enabled, inject a stub weakness report for testing
    if args.diagnostic:
        initial_state["weakness_report"] = WeaknessReport(
            weak_areas=["fundamentals", "data types"],
            recommended_topics=["1"],
        )

    final = graph.invoke(initial_state, config={"configurable": {"thread_id": "nereus-eval"}})

    inference = getattr(graph._tutor_agent, "_inference", None)
    llm_calls = getattr(inference, "calls", []) if inference is not None else []

    session = final.get("session")
    trace = {
        "model": inference.model_name() if inference is not None else "n/a",
        "profile": profile.model_dump(),
        "roadmap": [t.model_dump() for t in final["roadmap"].topics],
        "final_status": final["status"],
        "final_topic_index": final["current_topic_index"],
        "final_assessment": final["assessment"].model_dump() if final.get("assessment") else None,
        "session_brief": session.to_brief() if isinstance(session, LearningSession) else "",
        "llm_calls": llm_calls,
    }
    return trace


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="nereus-eval", description=__doc__)
    parser.add_argument("--skill", default="Python")
    parser.add_argument("--current-level", default="beginner", choices=[e.value for e in UserLevel])
    parser.add_argument(
        "--target-level", default="intermediate", choices=[e.value for e in UserLevel]
    )
    parser.add_argument("--hours", type=float, default=1.0)
    parser.add_argument("--deadline", type=int, default=30)
    parser.add_argument("--goal", default=None)
    parser.add_argument("--submission", default="this is good, I have mastered this material")
    parser.add_argument("--max-retries", type=int, default=2)
    parser.add_argument("--out", default="artifacts/run.jsonl")
    parser.add_argument("--log-level", default="INFO")
    parser.add_argument(
        "--diagnostic",
        action="store_true",
        help="Run with adaptive diagnostic roadmapping (Issue #7)",
    )
    parser.add_argument(
        "--dry-run", action="store_true", help="print trace to stdout, do not write file"
    )
    args = parser.parse_args(argv)

    logging.basicConfig(
        level=getattr(logging, args.log_level.upper(), logging.INFO),
        format="%(asctime)s %(levelname)s %(name)s | %(message)s",
    )

    trace = run(args)
    text = json.dumps(trace, default=str, indent=2)
    if args.dry_run:
        print(text)
    else:
        path = Path(args.out)
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("a", encoding="utf-8") as fh:
            fh.write(json.dumps(trace, default=str) + "\n")
        logger.info("trace written to %s", path)
        print(f"trace written to {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
