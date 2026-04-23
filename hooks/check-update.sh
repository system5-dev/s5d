#!/bin/sh
# Deprecated compatibility shim. Plugin hooks call `s5d update check --hook`
# directly; keep this file only for older installed plugin manifests.
exec s5d update check --hook
