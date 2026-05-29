  PHPUnit + Xdebug or PCOV for coverage.

  Composer:
    composer require --dev phpunit/phpunit phpunit/php-code-coverage

  Coverage driver — pick one (PCOV is faster):
    pecl install pcov                          # macOS / Linux
    # or: pecl install xdebug

    php -d pcov.enabled=1 vendor/bin/phpunit
    # or: XDEBUG_MODE=coverage vendor/bin/phpunit

  Threshold check (PHPUnit 11+):
    See phpunit.xml — <coverage><report><cobertura/><html/></coverage>
    Note: PHPUnit does not natively fail on threshold; use phpunit-coverage-check:
      composer require --dev rregeer/phpunit-coverage-check
      vendor/bin/coverage-check test-reports/php/coverage.xml {{THRESHOLD}}

  Run via skill:
    bash <skill>/scripts/run.sh --stacks php
