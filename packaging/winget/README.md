# pyenv-native Winget packaging

This folder holds generated or generator-backed Winget manifests for the Windows portable bundle.

## Generate manifests

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\build-winget-manifests.ps1
```

Common overrides:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\build-winget-manifests.ps1 `
  -GitHubRepo imyourboyroy/pyenv-native `
  -Tag vX.Y.Z `
  -ManifestVersion 1.12.0 `
  -Validate
```

## Optional validation

If `winget` is available locally, the script can run a strict manifest validation pass:

```powershell
pwsh -NoLogo -NoProfile -File .\scripts\build-winget-manifests.ps1 -Validate
```

## Output layout

The script writes manifests under:

```text
packaging/winget/manifests/<first-letter>/<publisher>/<package>/<version>/
```

For the default package identifier:

```text
packaging/winget/manifests/i/ImYourBoyRoy/pyenv-native/<version>/
```
