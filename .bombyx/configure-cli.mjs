import { configure } from "./configure.mjs";
import { configureCodex } from "./codex-config.mjs";
try {
  configure(
    process.env.HOME,
    process.argv[2],
    process.env.SILKBOARD_PUBLISH_ENDPOINT,
  );
  await configureCodex(process.env.HOME);
} catch (_) {
  console.error(
    "Silkboard VM configuration failed; check SILKBOARD_PUBLISH_ENDPOINT, private paths, existing configuration and the installed Codex app-server.",
  );
  process.exitCode = 1;
}
