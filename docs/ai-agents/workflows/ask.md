# ask

Find the most recent unresolved decision in the conversation. Reuse the
choices and recommendation already explained; do not invent a new decision.
Ask one focused interactive question and wait for its answer before dependent
work. On Codex use a supported question tool; on Claude use AskUserQuestion.
If no question UI is available, explain that limitation once and ask in chat.
If an asynchronous tool is used, do not treat timeout or silence as agreement.
No preceding decision means report that fact rather than manufacturing one.
