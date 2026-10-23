import { Marked } from "marked";
import DOMPurify from "dompurify";

// Used only in HTML text positions, never attributes.
const literal = (text: string) =>
  text.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
const parser = new Marked({
  async: false,
  gfm: true,
  renderer: {
    html: ({ text }) => literal(text),
    image: ({ text }) => literal(text),
  },
});

/** Sanitized Markdown nodes; raw HTML and image labels remain literal text. */
export function messageFragment(message: string): DocumentFragment {
  return DOMPurify.sanitize(parser.parse(message, { async: false }), {
    RETURN_DOM_FRAGMENT: true,
    ALLOWED_TAGS: [
      "p",
      "br",
      "strong",
      "em",
      "del",
      "blockquote",
      "ul",
      "ol",
      "li",
      "pre",
      "code",
      "h1",
      "h2",
      "h3",
      "h4",
      "h5",
      "h6",
      "a",
      "table",
      "thead",
      "tbody",
      "tr",
      "th",
      "td",
      "hr",
    ],
    ALLOWED_ATTR: ["href", "title", "start"],
    ALLOW_DATA_ATTR: false,
    ALLOW_ARIA_ATTR: false,
  });
}

/** Mount only sanitized nodes; message updates replace the previous contents. */
export function renderMarkdown(node: HTMLElement, message: string) {
  const update = (value: string) =>
    node.replaceChildren(messageFragment(value));
  update(message);
  return { update };
}

/** Compact plain-text preview of rendered content, without Markdown syntax. */
export function messagePreview(message: string): string {
  const text = (messageFragment(message).textContent || "")
    .replace(/\s+/g, " ")
    .trim();
  const chars = Array.from(text);
  return chars.length > 120
    ? chars.slice(0, 120).join("") + "…"
    : text || "View event";
}
