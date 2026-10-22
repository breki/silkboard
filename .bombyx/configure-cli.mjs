import { configure } from "./configure.mjs";
try {
  configure(
    process.env.HOME,
    process.argv[2],
    process.env.SILKBOARD_PUBLISH_ENDPOINT,
  );
} catch (_) {
  console.error(
    "Silkboard VM configuration failed; set SILKBOARD_PUBLISH_ENDPOINT to your HTTPS origin and inspect private paths/existing configuration.",
  );
  process.exitCode = 1;
}
