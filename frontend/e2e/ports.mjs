// Each concurrent worktree selects a separate eight-port fixture block.
export function fixturePort(
  logical,
  base = process.env.SILKBOARD_E2E_PORT_BASE ?? "38124",
) {
  if (
    !/^[1-9][0-9]{3,4}$/.test(base) ||
    Number(base) < 1024 ||
    Number(base) > 65528 ||
    !Number.isInteger(logical) ||
    logical < 38124 ||
    logical > 38131
  )
    throw new Error(
      "Expected fixture port base 1024–65528 and a logical port 38124–38131",
    );
  return Number(base) + logical - 38124;
}
