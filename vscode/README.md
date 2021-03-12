# prosemd

A VSCode extension for `prosemd`, an **experimental** proofreading and linting language server for
markdown files.  It aims to provide helpful and smart diagnostics when writing prose for technical
or non-technical documents alike.

**Note:** This extension and `prosemd` itself are still in a _Preview state_, which means that it's
neither feature complete nor entirely stable.

When you open markdown files the `prosemd` language server will start analysing it and providing
editor hints, warnings and errors depending on the stylistic or grammatical mistakes. Autofixing
them based on its suggestion is supported too.
