# Shell Completions

Enable tab completion for `ai-blame` commands, options, and file paths in your shell.

## Generating Completion Scripts

Use the `completions` subcommand to generate a completion script for your shell:

```bash
ai-blame completions <SHELL>
```

Supported shells: `bash`, `zsh`, `fish`, `elvish`, `powershell`

## Installation by Shell

### Zsh (with Oh My Zsh)

Oh My Zsh uses a custom completions directory:

```bash
# Create completions directory if it doesn't exist
mkdir -p ~/.oh-my-zsh/completions

# Generate and install the completion script
ai-blame completions zsh > ~/.oh-my-zsh/completions/_ai-blame

# Reload your shell
exec zsh
```

Alternatively, add to Oh My Zsh's custom directory:

```bash
mkdir -p ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/completions
ai-blame completions zsh > ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/completions/_ai-blame
exec zsh
```

### Zsh (without Oh My Zsh)

```bash
# Create a completions directory
mkdir -p ~/.zfunc

# Generate and install the completion script
ai-blame completions zsh > ~/.zfunc/_ai-blame

# Add to ~/.zshrc (if not already present):
echo 'fpath=(~/.zfunc $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc

# Reload your shell
source ~/.zshrc
```

### Bash

**On macOS with Homebrew:**

```bash
# Install bash-completion if not already installed
brew install bash-completion@2

# Generate and install the completion script
ai-blame completions bash > $(brew --prefix)/etc/bash_completion.d/ai-blame

# Add to ~/.bash_profile (if not already present):
echo '[[ -r "$(brew --prefix)/etc/profile.d/bash_completion.sh" ]] && . "$(brew --prefix)/etc/profile.d/bash_completion.sh"' >> ~/.bash_profile

# Reload your shell
source ~/.bash_profile
```

**On Linux:**

```bash
# System-wide installation (requires sudo)
ai-blame completions bash | sudo tee /etc/bash_completion.d/ai-blame > /dev/null

# Or user-local installation
mkdir -p ~/.local/share/bash-completion/completions
ai-blame completions bash > ~/.local/share/bash-completion/completions/ai-blame

# Reload your shell
source ~/.bashrc
```

### Fish

```bash
# Generate and install the completion script
ai-blame completions fish > ~/.config/fish/completions/ai-blame.fish

# Fish picks this up automatically, or reload with:
source ~/.config/fish/completions/ai-blame.fish
```

### PowerShell

```powershell
# Generate the completion script
ai-blame completions powershell > ai-blame.ps1

# Add to your PowerShell profile
# First, find your profile path:
echo $PROFILE

# Then add the completion script to your profile:
Add-Content $PROFILE "`n. path\to\ai-blame.ps1"

# Reload PowerShell
```

### Elvish

```bash
ai-blame completions elvish > ~/.elvish/lib/ai-blame.elv
# Then add: use ai-blame to ~/.elvish/rc.elv
```

## What Gets Completed

Once installed, tab completion provides:

| Context | Completion |
|---------|------------|
| `ai-blame <TAB>` | Subcommands: `init`, `report`, `annotate`, `stats`, `blame`, `timeline`, `transcript`, `completions` |
| `ai-blame blame <TAB>` | File paths |
| `ai-blame report --<TAB>` | Options: `--trace-dir`, `--config`, `--pattern`, etc. |
| `ai-blame transcript <TAB>` | Sub-subcommands: `list`, `view` |
| `ai-blame init --flavor <TAB>` | Values: `sidecar`, `in-place` |
| `ai-blame completions <TAB>` | Shells: `bash`, `zsh`, `fish`, `elvish`, `powershell` |

## Troubleshooting

### Completions not working after installation

1. Make sure you've reloaded your shell (`exec zsh`, `source ~/.bashrc`, etc.)
2. For zsh, ensure `compinit` is called after updating `fpath`
3. Check that the completion file was written correctly:
   ```bash
   # Should show completion function definition
   head -20 ~/.zfunc/_ai-blame  # or wherever you installed it
   ```

### Oh My Zsh: completions directory doesn't exist

Oh My Zsh may not create the completions directory by default. Create it manually:

```bash
mkdir -p ~/.oh-my-zsh/completions
```

### Completions outdated after upgrade

Regenerate the completion script after upgrading `ai-blame`:

```bash
ai-blame completions zsh > ~/.oh-my-zsh/completions/_ai-blame
exec zsh
```

### Permission denied errors

For system-wide installation on Linux, use `sudo`:

```bash
ai-blame completions bash | sudo tee /etc/bash_completion.d/ai-blame > /dev/null
```

## Updating Completions

When `ai-blame` adds new commands or options, regenerate your completion script:

```bash
# For zsh/Oh My Zsh:
ai-blame completions zsh > ~/.oh-my-zsh/completions/_ai-blame && exec zsh

# For bash:
ai-blame completions bash > $(brew --prefix)/etc/bash_completion.d/ai-blame && source ~/.bash_profile

# For fish:
ai-blame completions fish > ~/.config/fish/completions/ai-blame.fish
```
