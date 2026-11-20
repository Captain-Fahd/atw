# Starting the atw build in Claude Code

## One-time setup

1. Create a private GitHub repo `atw` and clone it, for example to `~/Projects/atw`. Make sure `git push` works from your machine (SSH key or `gh auth login`).
2. On GitHub, protect `main`: Settings → Branches → require a pull request before merging. That way Claude Code's pushes can only reach the phase branches.
3. Copy `CLAUDE.md` to the repo root and `docs/PLAN.md` to `docs/`.
4. Export the spec doc from claude.ai as Markdown and save it as `docs/SPEC.md`. (The diagrams may not export; PLAN.md has a text version of the process flow.)
5. Commit and push those three files to `main` yourself: `git add -A && git commit -m "docs: add spec, plan, Claude instructions" && git push`.
6. Run `claude` from the repo root.

At each STOP: review the phase branch on GitHub, open a pull request into `main`, and merge it once you're happy.

## Prompt 1: paste to start Phase 1

```
Build atw as described in CLAUDE.md. First check that docs/SPEC.md exists; if it doesn't, stop.
Read CLAUDE.md, the section headings of docs/SPEC.md (headings only), and only the "Phase 1" section of docs/PLAN.md.
Then reply with a Phase 1 plan of 15 lines or fewer, plus any questions, and wait for my OK.
After my OK, carry out Phase 1 up to STOP 1. Don't start Phase 2.
```

## Prompt 2: paste at the start of every later phase (in a fresh session)

Run `/clear` first, or start a new session.

```
Resume atw. Read CLAUDE.md and docs/PROGRESS.md.
My hardware results for the previous phase are in docs/hw/phase<N>/. Analyse them and summarise in 10 lines or fewer, including anything that blocks the next phase.
Then read only the "Phase <N+1>" section of docs/PLAN.md, reply with a plan of 15 lines or fewer, and wait for my OK.
```

## Keeping token use down

- One phase per session; use `/clear` between phases. `PROGRESS.md` carries state over, so Claude Code doesn't have to re-explore the codebase.
- If a session runs long within a phase, run `/compact`.
- Approve or correct the short plan before Claude Code starts. Redirecting it early is the cheapest kind of fix.
- Paste only the relevant error lines into the chat, not full logs.
