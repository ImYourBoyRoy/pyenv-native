# ./scripts/install-cursor-skills.ps1
# Back-compat wrapper — prefer install-agent-skills.ps1
& (Join-Path $PSScriptRoot 'install-agent-skills.ps1') -Agent cursor @args
