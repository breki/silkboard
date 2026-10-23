import { refresh } from "./refresh.mjs";
import { refreshGithub } from "./github-auth.mjs";

export function refreshSecrets(home, envFile, github = refreshGithub) {
  const failures = [];
  for (const [operation, message] of [
    [
      refresh,
      "Silkboard publisher secret refresh failed; inspect private input/configuration.",
    ],
    [
      github,
      "Silkboard GitHub CLI secret refresh failed; inspect private input/configuration or network.",
    ],
  ]) {
    try {
      operation(home, envFile);
    } catch (_) {
      failures.push(message);
    }
  }
  return failures;
}
