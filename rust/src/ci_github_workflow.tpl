name: s5d
on:
  pull_request:
  push:
    branches: [main, master]
permissions:
  contents: read
jobs:
  s5d:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v5
      - name: Install s5d v__S5D_VERSION__ (pinned release binary)
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          gh release download v__S5D_VERSION__ --repo system5-dev/s5d --pattern s5d-linux-x86_64 --output "$RUNNER_TEMP/s5d"
          chmod +x "$RUNNER_TEMP/s5d"
          echo "$RUNNER_TEMP" >> "$GITHUB_PATH"
      - name: S5D built-in checks (validate, architecture, drift)
        run: s5d ci exec
