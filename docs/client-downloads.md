# Reporting client downloads

Issue [#129](https://github.com/breki/silkboard/issues/129) adds standalone
`silkboard-publish` downloads to future eligible signed releases. Existing
tags are not retroactively packaged. A download needs no source checkout,
Rust, Cargo, Node, Git, PowerShell helper process or background service to run.

## Supported download targets

| Asset suffix | Supported runtime |
| --- | --- |
| `x86_64-unknown-linux-gnu` | x86-64 Linux with glibc 2.35 or newer; built on Ubuntu 22.04 |
| `x86_64-pc-windows-msvc.exe` | Windows 10/11 x64, private local NTFS storage; built on Windows Server 2022 with the C runtime linked statically |

Linux musl/Alpine, older glibc, Windows ARM64 and downloadable Linux ARM64
are outside this initial matrix. Linux ARM64 source builds remain separate.
TLS and public root certificates are compiled in. Both downloads support
ordinary events and explicit workflow-plan commands. Native Codex hook setup
remains a separate operator integration; downloading does not install hooks.

## Verify and install on Linux

Select an eligible tag from [GitHub Releases](https://github.com/breki/silkboard/releases).
Replace `vX.Y.Z` below with that tag. Download its client and `SHA256SUMS`
into a new temporary directory:

```sh
(
set -eu
tag=vX.Y.Z
asset="silkboard-publish-$tag-x86_64-unknown-linux-gnu"
base="https://github.com/breki/silkboard/releases/download/$tag"
curl --fail --location --output "$asset" "$base/$asset"
curl --fail --location --output SHA256SUMS "$base/SHA256SUMS"
# Require this exact file to match before installing or running it.
awk -v file="$asset" '$2 == file' SHA256SUMS > selected.sha256
test "$(wc -l < selected.sha256)" -eq 1
sha256sum --check selected.sha256
install -d -m 0755 "$HOME/.local/bin"
install -m 0755 "$asset" "$HOME/.local/bin/silkboard-publish"
"$HOME/.local/bin/silkboard-publish" --version
)
```

Ensure `$HOME/.local/bin` is on `PATH`. If any download or verification fails,
stop before installation. Checksums detect corruption; checksums downloaded
from the same release do not independently authenticate a compromised GitHub
account or release. Confirm the selected release's signed-tag/full-suite
evidence and compare the printed tag and full commit with that revision.
Version output is provenance metadata, not a signature verifier.

## Verify and install on Windows

Use PowerShell for installation; publishing itself does not invoke it.
Download in a new temporary directory and stop on verification failure:

```powershell
$ErrorActionPreference = 'Stop'
$tag = 'vX.Y.Z'
$asset = "silkboard-publish-$tag-x86_64-pc-windows-msvc.exe"
$base = "https://github.com/breki/silkboard/releases/download/$tag"
Invoke-WebRequest "$base/$asset" -OutFile $asset
Invoke-WebRequest "$base/SHA256SUMS" -OutFile SHA256SUMS
$pattern = '^[0-9a-f]{64}  ' + [regex]::Escape($asset) + '$'
$entry = @(Get-Content SHA256SUMS | Where-Object { $_ -match $pattern })
if ($entry.Count -ne 1) { throw 'Missing or ambiguous checksum' }
$expected = ($entry[0] -split '  ')[0]
if ((Get-FileHash $asset -Algorithm SHA256).Hash -ine $expected) {
    throw 'Client checksum mismatch'
}
$bin = Join-Path $env:LOCALAPPDATA 'Silkboard\bin'
New-Item -ItemType Directory -Force $bin | Out-Null
Copy-Item $asset (Join-Path $bin 'silkboard-publish.exe')
& (Join-Path $bin 'silkboard-publish.exe') --version
```

Use that full executable path or add `$bin` to your user `PATH`. Compare the
tag and full commit with the selected release as described above. The Windows
download is not Authenticode-signed; do not treat Windows publisher identity
prompts as cryptographic release verification. No administrator install or
Visual C++ redistributable installation is needed for the static CRT build.

## Configure a different project

The operator supplies a server HTTPS origin, project slug and project-scoped
publish-only token through an authorized private channel. Installation grants
no permission to obtain credentials, access a dashboard or execute tasks.
The server must already support the requested event/plan API. See the
[configuration and transport contract](publishing-cli.md#private-configuration)
and [workflow-plan commands](task-plan-publisher.md).

Linux uses the mode-0700 directories and mode-0600 files documented there.
On Windows, prepare a fresh private directory on local NTFS before placing
any configuration or credential in it:

```powershell
$root = Join-Path $env:LOCALAPPDATA 'Silkboard\publisher'
New-Item -ItemType Directory -Path $root | Out-Null
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
$acl = [Security.AccessControl.DirectorySecurity]::new()
$acl.SetOwner($sid)
$acl.SetAccessRuleProtection($true, $false)
$rule = [Security.AccessControl.FileSystemAccessRule]::new(
    $sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
$acl.AddAccessRule($rule)
Set-Acl -LiteralPath $root -AclObject $acl
$state = Join-Path $root 'state'
New-Item -ItemType Directory -Path $state | Out-Null
# Copy the operator-delivered token privately; never put its value in commands.
Copy-Item 'C:\secure-transfer\publisher.token' (Join-Path $root 'publisher.token')
$config = @{
    endpoint = 'https://silkboard.example.test'
    project = 'your-project'
    token_file = (Join-Path $root 'publisher.token')
    state_dir = $state
} | ConvertTo-Json
[IO.File]::WriteAllText((Join-Path $root 'config.json'), $config,
    [Text.UTF8Encoding]::new($false))
```

Replace the example origin and project with the operator's values. The token
file contains exactly 64 lowercase hexadecimal characters, optionally followed
by LF; save without a BOM or CRLF. Do not reuse a populated directory for this
setup or reset permissions/counters during an active session.

Windows accepts DACL grants only to the current user, SYSTEM and built-in
Administrators, and requires one of those identities to own each checked
object. Null DACLs, other grantees, unsupported ACE types, hardlinked files,
reparse-point/junction paths and UNC storage fail closed. Private configuration,
token and state directories must remain under operator-controlled ancestors.
Do not store them on network shares, FAT/exFAT or synchronized/cloud folders.
State replacement flushes the new file before and after rename; Windows has
no Unix directory-fsync guarantee. Abrupt power-loss recovery is not claimed.
Keep working state through client upgrades, and never delete counters to
repair a publishing error.

## Basic reporting smoke checks

Use the installed executable, private config and a fresh actual native session
identity. For a deliberate manual CLI smoke check, generate a fresh identity
(for example `manual-smoke-` plus a new UUID) and keep it for the related event
and plan reports. Do not impersonate another agent's native session.

On Linux, pipe JSON into:

```sh
silkboard-publish publish --config "$HOME/.config/silkboard-publish/config.json"
```

On Windows, pipe a JSON string into:

```powershell
$client = Join-Path $env:LOCALAPPDATA 'Silkboard\bin\silkboard-publish.exe'
$configPath = Join-Path $root 'config.json'
$session = 'manual-smoke-' + [guid]::NewGuid().ToString()
@{ hostname = $env:COMPUTERNAME; tool = 'manual'; session_id = $session
   message = 'Checking installed reporting client' } |
    ConvertTo-Json -Compress | & $client publish --config $configPath
if ($LASTEXITCODE -ne 0) { throw 'Event delivery unconfirmed' }
```

The event JSON has `hostname`, `tool`, `session_id` and `message` fields. Exit
0 and `Progress acknowledged` confirm its exact event acknowledgement.
Repeat with the same identity to check preserved counters.

For workflows, generate a fresh UUID v4 task ID and use the complete
[plan input](task-plan-publisher.md#publish-a-complete-current-plan) with your actual
reporter identity. Pipe it into `plan start --config PATH`, then pipe the same
task and reporter into `plan publish --config PATH`. This works on both
platforms; PowerShell can pipe `ConvertTo-Json -Depth 10 -Compress` into
`& $client plan start --config $configPath` or `plan publish`.
Exit 0 and `Task plan acknowledged` confirm each plan acknowledgement.
Use `codex-plan` only after the separate operator opt-in and compatible native
identity integration. An unsuccessful attempt can have reached the server;
do not restart an allocated plan or delete its working state to retry.

All delivery remains publish-only, bounded and best-effort, with no retry
queue, server task fetching or execution authority.

## Release pipeline and evidence limits

The existing release workflow and updater retain their exact three-job
contract. A separate `workflow_run` pipeline accepts only a successful push
run of registered Release workflow `387356305` at its agreed path, then
rechecks the signed tag against the exact upstream commit. The
`client-release-check` command reuses the updater's evaluator to verify the
exact latest run attempt, repository, tag-bound reusable CI revision and all
three successful jobs; skipped, missing or additional jobs fail. It rereads
the run after fetching jobs to detect a changed attempt. Read-only native
jobs build, lint and smoke-test both clients. Their tests copy the packaged
executable to a disposable directory and clear `PATH`, publish loopback events
and plans across process restarts, and test Windows private storage. Only a
read-only job repeats admission after both client builds. Only the later
publication job receives `contents: write`; it checks that the exact admitted
signed tag object and successful upstream attempt remain unchanged, creates
a draft release, uploads both clients plus `SHA256SUMS`, then
publishes it. Existing releases/assets are never overwritten implicitly.
If publication fails, any partial draft requires operator inspection before
a retry; deleting an existing draft or replacing assets is not automatic.

PR jobs exercise the same platform build/smoke path without publication
permissions. Their binaries explicitly identify development provenance.
Source builds and server validation/in-guest builds remain unchanged. A
successful server release can deploy independently of later client packaging;
client packaging failure cannot make an unverified server release eligible.
Defining these jobs does not establish native Windows execution, public asset
availability or a production deployment. Those require observed CI and a
future separately authorized release.
