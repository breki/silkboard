# Controls with ten projects and twenty agents

Revised proposal for [#122](https://github.com/breki/silkboard/issues/122),
following the owner's 2026-11-03 feedback. This is an offline, interactive
design prototype with fictional reports, not an application change or an
approved design. No production access or external data is used.

Open [proposal.html](proposal.html) in a browser with its adjacent CSS and
JavaScript files. The prototype needs no server, build step, or network.
Download the whole folder when viewing through GitHub; GitHub's source viewer
does not run the HTML. Use the screenshots below for immediate review.

To try the proposal, select a project, choose an agent, then expand Report &
identity. Clear filters to return to all twenty agents. Switch to Events and
open All time to try a preset or enter a custom range. Use the header's theme
button to compare Light and Dark.

## Proposed direction

Ten projects fit in a quiet desktop index instead of a row of large filter
buttons. Each project has two main agent sessions; all twenty cards are
present, sorted by their latest received report. Cards retain project,
tool/host, report, received time and a disclosure for exact native identity.
The reported workflow state is not a connection or availability indicator.

On phones, the project index becomes a searchable chooser. The agent chooser
searches task name, project, tool and host, and respects the selected project.
Project selection clears the previous agent selection. Counts show the scope
of the result; empty results explain how to recover. Project and agent
filtering on Agents is a proposal, beyond the current application's controls.

The Events toolbar replaces two permanently visible native date/time fields
with one **Received time** range control. Opening it offers All time, Last
hour, Last 24 hours and Last 7 days. A custom range uses paired date and time
fields with explicit formats, one Apply range action and Cancel. There is no
calendar grid in this iteration: presets serve quick narrowing, while typed
dates and 24-hour times serve precise ranges without a browser-specific
picker. This is a concrete direction for review, not a settled product choice.

Presets apply immediately; custom edits apply together. Invalid dates, invalid
times and reversed ranges leave the previous filter active. Cancel and Escape
discard unapplied edits. Modal dialogs keep keyboard focus inside and return
it to their trigger. The prototype uses a fixed sample clock of 2026-11-03
12:00 UTC, so screenshots and preset results remain reproducible. Inclusive
minute endpoints here mean exact HH:mm:00 timestamps, not the whole ending
minute. Production timezone handling and range semantics need to preserve the
application's contract; this UTC fixture does not implement local DST behavior.

## Review images

| Situation                         | Light                                | Dark                                 |
| --------------------------------- | ------------------------------------ | ------------------------------------ |
| All twenty agents, desktop        | [Overview](agents-desktop-light.png) | [Overview](agents-desktop-dark.png)  |
| All twenty agents, phone viewport | [390 px](agents-phone-light.png)     | [320 px](agents-phone-320-dark.png)  |
| Time range open                   | [Desktop](range-desktop-light.png)   | [320 px phone](range-phone-dark.png) |

- [Searchable project chooser on a phone](project-chooser-phone.png)
- [Selected agent with exact identity expanded](filtered-agent-phone.png)
- [Events with all twenty received reports](events-desktop-light.png)
- [Applied custom range on a phone](custom-range-phone.png)
- [Invalid range with feedback on a short phone](invalid-range-phone.png)

Desktop overview screenshots include the full scrollable page. Phone images
show a viewport; all twenty cards remain available by scrolling the prototype.
Agents and Events navigation, filters, identity disclosures and theme switching
work locally. Workflows and Todos are static navigation context; this prototype
does not revise their behavior. It does not implement routing/deep-link #140,
backend queries, pagination, authentication, persistence or automatic polling.

Earlier research and styling references remain in the
[original proposal](https://github.com/breki/silkboard/tree/9adc31f27ed57c6d4caa649ca09630888a8fc3ce/docs/mockups/control-style-study-2026-10-26).
No external UI library, font, image or new dependency is introduced here.

## Reproduce and check

From the repository root with its admitted frontend dependencies and installed
Playwright Chromium:

```sh
node docs/mockups/controls-at-scale/render.mjs
```

If using an already installed sibling frontend, set
`SILKBOARD_MOCKUP_FRONTEND` to that frontend directory. The renderer writes
only this folder's eleven PNGs and [render-checks.json](render-checks.json).
It fails on page errors, missing fixture records, horizontal page/dialog
overflow or visible phone controls smaller than 44 by 44 CSS pixels in the
captured scenarios. It exercises project/agent search and selection, exact
identity disclosure, clear filters, view navigation, a recent preset, custom
range validation, cancellation, Escape/focus return and theme switching.
Keyboard regressions cover project selection and clearing filters without
losing focus, unique From/To field names, and reverse Tab wrapping in the
range dialog. Independent review also exercised forward/reverse wrapping in
the range dialog and the empty chooser.

The checks are local prototype evidence. They do not establish production
behavior, a full accessibility audit, screen-reader compatibility, OS keyboard
behavior on a real phone or owner acceptance. Full application and release
validation are separate from rendering these design artifacts. Review the
layout and time-range interaction before any application implementation.
