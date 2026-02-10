# jarvis-linux-sandbox

This crate is responsible for producing:

- a `jarvis-linux-sandbox` standalone executable for Linux that is bundled with the Node.js version of the jarvis CLI
- a lib crate that exposes the business logic of the executable as `run_main()` so that
  - the `jarvis-exec` CLI can check if its arg0 is `jarvis-linux-sandbox` and, if so, execute as if it were `jarvis-linux-sandbox`
  - this should also be true of the `jarvis` multitool CLI
