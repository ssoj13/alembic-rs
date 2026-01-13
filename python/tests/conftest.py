"""Pytest configuration and fixtures for alembic-rs Python tests."""

import os
import sys
import pytest
import tempfile
from pathlib import Path


@pytest.fixture(scope="session")
def test_data_dir():
    """Return the test data directory."""
    return Path(__file__).parent.parent / "data"


@pytest.fixture
def temp_abc_file():
    """Create a temporary .abc file path."""
    with tempfile.NamedTemporaryFile(suffix=".abc", delete=False) as f:
        path = f.name
    yield path
    # Cleanup
    if os.path.exists(path):
        os.remove(path)


@pytest.fixture
def temp_dir():
    """Create a temporary directory for test files."""
    with tempfile.TemporaryDirectory() as d:
        yield Path(d)
