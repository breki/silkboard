import { refresh } from "./refresh.mjs";
try {
  refresh(process.env.HOME, process.env.BOMBYX_ENV_FILE);
} catch (_) {
  console.error(
    "Silkboard publisher secret refresh failed; inspect private input/configuration.",
  );
  process.exitCode = 1;
}
