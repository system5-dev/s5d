  bats-core (SoTA shell testing).

  Install:
    brew install bats-core
    # or:
    git clone https://github.com/bats-core/bats-core.git
    sudo ./bats-core/install.sh /usr/local

  Optional libs:
    git submodule add https://github.com/bats-core/bats-support.git test/test_helper/bats-support
    git submodule add https://github.com/bats-core/bats-assert.git  test/test_helper/bats-assert

  Coverage (kcov, Linux + macOS):
    brew install kcov   # macOS
    kcov --include-path=$(pwd) coverage bats tests/*.bats

  Run via skill:
    bash <skill>/scripts/run.sh --stacks shell
