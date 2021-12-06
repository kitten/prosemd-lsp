<p align="center">
  <img
    src="https://raw.githubusercontent.com/kitten/prosemd-lsp/main/vscode/icon.png"
    alt="prosemd"
    width="200">
</p>

`prosemd` is an **experimental** proofreading and linting language server for markdown files.
It aims to provide helpful and smart diagnostics when writing prose for technical or non-technical
documents alike.

Under the hood, it integrates with any editor supporting the [Language Server
Protocol](https://microsoft.github.io/language-server-protocol/), and uses
[nlprule](https://github.com/bminixhofer/nlprule), which is based on
[LanguageTool](https://github.com/languagetool-org/languagetool), to highlight possible errors and
provides suggestions on how to address them.

> **Note:** On the roadmap for more features are other useful linting rules, related to Markdown
> links, formatting, and other potentially helpful features, however for now `prosemd` is limited to
> just grammar & style correction with `nlprule` and only supports English.

## Quick Start

### Setup in VSCode

**If you're using VSCode, all you have to do is install the `prosemd` extension.**

[Install the extension from the VSCode Marketplace.](https://marketplace.visualstudio.com/items?itemName=kitten.prosemd)

### Manual Installation

If you're setting the language server up with your editor of choice then you'll need to either
download the executable or compile it from source:

- Download the [latest release executable](https://github.com/kitten/prosemd-lsp/releases) for your
  OS (Either: `prosemd-lsp-linux`, `prosemd-lsp-windows.exe`, or `prosemd-lsp-macos`).
- or; install [Rust](https://www.rust-lang.org/tools/install) and then run
  `cargo install prosemd-lsp` to compile `prosemd` from source.

### Configuring [`coc.nvim`](https://github.com/neoclide/coc.nvim)

[First, make sure that you install the `prosemd-lsp` executable.](#manual-installation)

You may add `prosemd` to `coc.nvim`'s config manually in `coc-settings.json` opened by the
`:CocConfig` command, like so:

```json
{
  "languageserver": {
    "prosemd": {
      "command": "/usr/local/bin/prosemd-lsp",
      "args": ["--stdio"],
      "filetypes": ["markdown"],
      "trace.server": "verbose",
      "settings": {
        "validate": true
      }
    }
  }
}
```

Don't forget to swap out the binary's path at `command` to where you've installed the `prosemd-lsp`
executable.

### Configuring [`nvim-lspconfig`](https://github.com/neovim/nvim-lspconfig)

[First, make sure that you install the `prosemd-lsp` executable.](#manual-installation)

You may add `prosemd` to [Neovim's built-in language server
client](https://neovim.io/doc/user/lsp.html) by adding it to `nvim-lspconfig`'s list of language
servers:

```lua
local lsp_configs = require('lspconfig.configs')

lsp_configs.prosemd = {
  default_config = {
    -- Update the path to prosemd-lsp
    cmd = { "/usr/local/bin/prosemd-lsp", "--stdio" },
    filetypes = { "markdown" },
    root_dir = function(fname)
      return lsp_util.find_git_ancestor(fname) or vim.fn.getcwd()
    end,
    settings = {},
  }
}

-- Use your attach function here
local lsp = require('lspconfig')
lsp.prosemd.setup{ on_attach = on_attach }
```

Don't forget to swap out the binary's path at `cmd` to where you've installed the `prosemd-lsp`
executable.
