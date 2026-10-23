// Disposable local credentials, never used outside the browser test server.
export const password = "silkboard browser fixture password";
export const hostile = '<img src=x onerror="window.injected=true">';

export const markdownReport =
  "**Markdown report** *Ready* with `cargo test`. [Reference](https://example.invalid/reference) ![tracker](https://example.invalid/tracker.png) <script>window.injected=true</script> [unsafe](javascript:alert(1)) `" +
  "long-code-line-".repeat(30) +
  "`";
