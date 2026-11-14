# ADR 0010: Multi-Format Session Replay and Visual Terminal Export

* **Status**: Accepted
* **Deciders**: Nereus Engineering Core, Project Maintainer
* **Date**: 2026-11-14

---

## Context & Problem Statement

Learners want to share their programming breakthroughs, review learning journeys, and maintain verifiable proof of concept mastery. However, terminal session logs are traditionally plain unstructured text or binary shell logs that cannot be easily inspected or embedded in portfolios, READMEs, or documentation.

## Decision Drivers

* Human-readable, standalone Markdown summary reports.
* Animated terminal session replay compatible with standard terminal players (e.g., [Asciinema](https://asciinema.org/)).
* Self-contained, responsive dark-mode vector SVG badges that render crisp terminal frames anywhere (GitHub READMEs, blog posts, portfolios) without external JavaScript dependencies.
* Easy-to-use CLI interface (`nereus sessions export <session-id> --format <format>`).

## Considered Options

1. **Native PNG/GIF Screenshot Generation via Headless Chromium**: Heavy external dependencies (Puppeteer / Chromium), slow generation (~3-5s), and large file sizes.
2. **Raw Terminal Output Log (`script` / `typescript`)**: Minimal portability and lacking structured metadata (timestamps, agent dialogues, scores).
3. **Multi-Format Native Generator**:
   - `Markdown`: Structured table of topics, assessments, scores, and feedback.
   - `Asciinema v2 (.cast)`: JSON newline-delimited event stream of timed terminal escape sequences.
   - `Vector SVG`: Self-contained dark-mode SVG rendering terminal chrome, typography, and syntax highlighting.

## Decision Outcome

Chosen option: **Multi-Format Native Generator** in `nereus-cli::export`.

Implemented natively in Rust without headless browser dependencies:
- `--format markdown`: Renders session overview, learner level, score metrics, and chronological assessment logs.
- `--format cast`: Generates standard Asciinema v2 header `{"version": 2, "width": 100, "height": 30, ...}` followed by timed stdout frame writes.
- `--format svg`: Generates clean, responsive SVG with macOS window controls, monospace typography, status pill, and styled session statistics.

### Consequences

* **Positive**: Zero external binary dependencies; generates exports in <5 milliseconds.
* **Positive**: Asciinema `.cast` files can be played with `asciinema play` or shared online.
* **Positive**: SVG exports can be embedded directly in GitHub README files and documentation.
* **Negative**: Complex terminal control sequences (e.g., dynamic cursor movements) in SVG are rendered as static summary snapshots rather than arbitrary live animations.
