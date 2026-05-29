  Go test is built into the toolchain. No install needed.

  Coverage reporting:
    go test -coverprofile=cover.out ./...
    go tool cover -html=cover.out -o coverage.html

  Optional — LCOV export for Codecov / VS Code:
    go install github.com/jandelgado/gcov2lcov@latest
    gcov2lcov -infile cover.out -outfile coverage.lcov

  Optional — JUnit XML for CI:
    go install github.com/jstemmer/go-junit-report/v2@latest
    go test -v ./... | go-junit-report -set-exit-code > junit.xml

  Run with reports:
    bash <skill>/scripts/run.sh --stacks go
