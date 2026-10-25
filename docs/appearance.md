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
