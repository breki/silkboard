# Appearance

Choose **Theme** in the navigation menu: **System**, **Light** or **Dark**.
The first-owner setup page exposes the same selector directly in its header,
without adding navigation or requiring sign-in.

System is the default and follows the browser or operating system appearance,
including changes while the page is open. Light and Dark override that setting.
The choice is saved in this browser and survives reloads and navigation; it
is not an account preference synchronized across devices. If browser settings
block storage, switching still works in the current page, but a reload can
return to System.

The shared palette covers reports and Markdown, task graphs and step states,
received history, dialogs, credentials, setup and application status. Selection
and focus remain visible, and graph states retain their labels and symbols.
Theme changes do not change authentication or publishing permissions.

This implements [#99](https://github.com/breki/silkboard/issues/99).
[The Workflows rename](https://github.com/breki/silkboard/issues/98) is separate;
this appearance change does not rename routes, commands or existing UI labels.

## Controls and action hierarchy

Commit actions such as Sign in, Add todo and Save text use the green primary
style. Ordinary actions have a neutral face; maintenance and row actions use
quieter text treatments. Destructive actions retain explicit labels and
confirmation, with a distinct alert color. Status badges remain information,
while destination links retain native navigation behavior.

Desktop buttons use compact 32 px base faces and 14 px labels. Narrow screens
and coarse pointers retain at least 44 px button targets; the painted face is
inset inside that native clickable area. Primary phone actions can be larger.
Text can wrap for long labels, and enlarged text increases the controls rather
than being clipped. Keyboard focus outlines surround the actual target.
Graph nodes, event summaries and session-group disclosures keep their own
content surfaces and existing interactions.

On phones the header separates the brand/menu row from the visible primary
page links. The current destination has a visible indicator and is announced
as the current page. Setup retains its navigation-free header. Light, Dark
and System preferences continue to apply to controls and focus states.

This control styling implements [#122](https://github.com/breki/silkboard/issues/122).
Four-page navigation comes from #121. The keyboard editor (#124) remains
separate; existing todo editing and deletion behavior is retained.

The owner approved the revised ten-project/twenty-agent proposal in
[PR #142](https://github.com/breki/silkboard/pull/142) on 2026-11-03. Its quieter
neutral/green palette, compact cards with identity disclosures, desktop
project index, searchable phone choices and received-time panel supersede the
earlier styling-only direction. The [feed guide](owner-feed.md) describes the
working controls. This implementation preserves local time, System appearance,
existing navigation and owner-only reads; the proposal's UTC sample clock is
not a production setting. A repository implementation does not establish a
release, live deployment or owner phone acceptance.
