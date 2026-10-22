import { configure } from "./configure.mjs";
try {
  configure(process.env.HOME, process.argv[2]);
} catch (_) {
  console.error(
    "Silkboard VM configuration failed; inspect private paths and existing configuration.",
  );
  process.exitCode = 1;
}
