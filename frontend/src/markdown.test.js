import { expect, test } from "vitest";
import { messageFragment, messagePreview, renderMarkdown } from "./markdown";

test("renders common Markdown with compact readable previews", () => {
  const source =
    "# Report\n\n**Ready** and *reviewed* with `cargo test`.\n\n- First\n- Second\n\n> quoted\n\n```sh\necho '<safe>'\n```\n\n[Docs](https://example.com/guide \"Guide\")\n\n| Check | Result |\n| --- | --- |\n| Tests | Pass |";
  const fragment = messageFragment(source);
  expect(fragment.querySelector("h1").textContent).toBe("Report");
  expect(fragment.querySelector("strong").textContent).toBe("Ready");
  expect(fragment.querySelector("em").textContent).toBe("reviewed");
  expect(fragment.querySelectorAll("li")).toHaveLength(2);
  expect(fragment.querySelector("blockquote").textContent).toContain("quoted");
  expect(fragment.querySelector("pre code").textContent).toContain(
    "echo '<safe>'",
  );
  expect(fragment.querySelector("a").getAttribute("href")).toBe(
    "https://example.com/guide",
  );
  expect(fragment.querySelector("a").getAttribute("title")).toBe("Guide");
  expect(fragment.querySelector("table td").textContent).toBe("Tests");
  expect(messagePreview("**Ready** with `cargo test`.\n\nNext step")).toBe(
    "Ready with cargo test. Next step",
  );
  expect(messagePreview("😀".repeat(130))).toBe("😀".repeat(120) + "…");
  expect(messagePreview("")).toBe("View event");
  expect(messagePreview("---")).toBe("View event");
});

test.each([
  "javascript:alert(1)",
  "JaVaScRiPt:alert(1)",
  "vbscript:msgbox(1)",
  "data:text/html;base64,PHNjcmlwdD4=",
  "file:///etc/passwd",
  "jav&#x61;script:alert(1)",
  "javascript&#58;alert(1)",
])("unsafe link destination %s cannot become an active link", (url) => {
  expect(
    messageFragment(`[click](${url})`).querySelector("a[href]"),
  ).toBeNull();
});

test("raw HTML, images and embedded resource attempts stay inert", () => {
  const attack =
    '<img src="https://example.com/tracker" onerror="alert(1)">\n\n<script>alert(1)</script>\n\n<iframe src="https://example.com"></iframe>\n\n<style>body{background:url(https://example.com)}</style>\n\n![image <label>](https://example.com/image.png)\n\n<svg onload="alert(1)"></svg>\n\n<video src="https://example.com/movie"></video>';
  const fragment = messageFragment(attack);
  expect(
    fragment.querySelector(
      "img,script,iframe,style,svg,video,source,link,object,embed,form,input",
    ),
  ).toBeNull();
  expect(fragment.textContent).toContain(
    '<img src="https://example.com/tracker"',
  );
  expect(fragment.textContent).toContain("image <label>");
  for (const element of fragment.querySelectorAll("*")) {
    expect(
      [...element.attributes].some((attr) =>
        /^(on|src|style|id|name)/i.test(attr.name),
      ),
    ).toBe(false);
  }
});

test("safe relative and mail links remain usable and updates remove old content", () => {
  const node = document.createElement("div");
  const action = renderMarkdown(
    node,
    "[local](/status) and [mail](mailto:dev@example.com)",
  );
  expect(
    [...node.querySelectorAll("a")].map((a) => a.getAttribute("href")),
  ).toEqual(["/status", "mailto:dev@example.com"]);
  action.update("**New message**");
  expect(node.querySelector("a")).toBeNull();
  expect(node.querySelector("strong").textContent).toBe("New message");
});
