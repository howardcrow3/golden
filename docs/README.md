# Golden planning docs

Golden is a desktop app (Mac and Windows) that reads a household's bank statements, card statements and paystubs, keeps an AI-maintained P&L and balance sheet, projects the household month by month, and shows when each person can retire. It runs locally on the CeeVee agent harness, with a Private mode where nothing leaves the computer.

These files are an export of the planning doc written 2026-11-15. The live versions, with diagrams, are:

- Doc (PRD, specification, visual design, architecture): https://claude.ai/artifact/CaJcMbnH76NnFZ3CP4rjW4
- Screen mock-ups (design canvas): https://claude.ai/artifact/2Mbh7hSkqnVbFCZbGrkR9q

| File | What it holds |
| --- | --- |
| [01-prd.md](01-prd.md) | Goals, principles, scope, requirements (IN, BK, PR, RT, IV, AI, MC, SC), success measures, open questions |
| [02-specification.md](02-specification.md) | Screens, chat and what-ifs, ingestion, ledger, projection, lumpy income and time grain, parameters, allocation, AI tasks, MCP, data model |
| [03-visual-design.md](03-visual-design.md) | Turtlebox (High Turtle) design rules, chart language, app shell and screen layouts |
| [04-architecture.md](04-architecture.md) | Stack, crates, AI layer, Plaid and licensing, security, testing, order of work, risks |
| [screens/](screens/) | HTML sources of the Overview, Retirement and Plan-with-chat mock-ups (design canvas format, sample figures only) |

The architecture diagram is a widget in the live doc and appears as a placeholder in 04-architecture.md.
