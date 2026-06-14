s5d:
  stage: test
  image: debian:bookworm-slim
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
  before_script:
    - apt-get update -qq && apt-get install -y -qq curl ca-certificates
    - curl -fsSL -o /usr/local/bin/s5d https://github.com/system5-dev/s5d/releases/download/v__S5D_VERSION__/s5d-linux-x86_64
    - chmod +x /usr/local/bin/s5d
  script:
    - s5d ci exec
