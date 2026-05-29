  RSpec + SimpleCov (SoTA Ruby).

  Add to Gemfile (group :development, :test):
    gem 'rspec'
    gem 'simplecov', require: false
    gem 'rspec_junit_formatter', require: false

  Then:
    bundle install
    bundle exec rspec --init   # generates .rspec and spec/spec_helper.rb

  Add to spec/spec_helper.rb (top, BEFORE requiring app code):
    require 'simplecov'
    SimpleCov.start { add_filter '/spec/'; minimum_coverage {{THRESHOLD}} }

  Run via skill:
    bash <skill>/scripts/run.sh --stacks ruby
