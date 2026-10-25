import type { FeedEvent } from "./feed-api";
// eslint-disable-next-line no-unused-vars -- Type-only callback parameter describes the event action.
export type EventToggle = (event: FeedEvent) => void;
// eslint-disable-next-line no-unused-vars -- Type-only callback parameter describes the session action.
export type GroupToggle = (key: string) => void;
export interface EventRow {
  event: FeedEvent;
  index: number;
}
export interface SessionGroup {
  key: string;
  name: string;
  identity: FeedEvent;
  events: EventRow[];
  children: SessionGroup[];
  first: number;
}
export const eventKey = (event: FeedEvent) =>
  JSON.stringify([event.project, event.event_id]);
export const sessionKey = (event: FeedEvent) =>
  JSON.stringify([
    event.project,
    event.hostname,
    event.tool,
    event.session_id,
    event.subagent_id,
  ]);

// A parent reference identifies a primary native session, never a display name
// or an arbitrary child sharing that native ID. Only this received page is used.
export function groupEvents(events: FeedEvent[]) {
  const groups = new Map<string, SessionGroup>();
  events.forEach((event, index) => {
    const key = sessionKey(event);
    let group = groups.get(key);
    if (!group) {
      group = {
        key,
        name: "",
        identity: event,
        events: [],
        children: [],
        first: index,
      };
      groups.set(key, group);
    }
    if (BigInt(event.sequence) > BigInt(group.identity.sequence))
      group.identity = event;
    group.events.push({ event, index });
  });
  const roots: SessionGroup[] = [];
  const unmatched: SessionGroup[] = [];
  for (const group of groups.values()) {
    const event = group.identity;
    group.name =
      event.session_name ??
      (event.subagent_id
        ? `Subagent ${event.subagent_id}`
        : `Session ${event.session_id}`);
    if (event.subagent_id === null) {
      roots.push(group);
      continue;
    }
    const parent = event.parent_session_id
      ? groups.get(
          sessionKey({
            ...event,
            session_id: event.parent_session_id,
            subagent_id: null,
          }),
        )
      : undefined;
    if (parent) {
      parent.children.push(group);
      parent.first = Math.min(parent.first, group.first);
    } else unmatched.push(group);
  }
  roots.sort((a, b) => a.first - b.first);
  return { roots, unmatched };
}
