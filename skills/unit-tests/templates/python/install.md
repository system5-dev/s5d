  Install (pytest + pytest-cov — SoTA for Python testing):
    uv add --dev pytest pytest-cov pytest-xdist
    # or: pip install pytest pytest-cov pytest-xdist
    # add to pyproject.toml [tool.uv] dev-dependencies = ["pytest", ...]

  Optional but recommended:
    pytest-asyncio       # async test support
    pytest-mock          # mock fixture
    hypothesis           # property-based testing

  Run with reports:
    bash <skill>/scripts/run.sh --stacks python
