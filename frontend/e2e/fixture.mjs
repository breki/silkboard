// Disposable local credentials, never used outside the browser test server.
export const password = "silkboard browser fixture password";
export const hostile = '<img src=x onerror="window.injected=true">';

export const markdownReport =
  "## Markdown report\n\n**Ready** and *reviewed* with `cargo test`.\n\n- First check\n- Second check\n\n[Reference](https://example.invalid/reference)\n\n![tracker](https://example.invalid/tracker.png)\n\n<script>window.injected=true</script>\n\n[unsafe](javascript:alert(1))\n\n```text\n" +
  "long-code-line-".repeat(30) +
  "\n```\n\n| Check | Result |\n| --- | --- |\n| Tests | Pass |";
