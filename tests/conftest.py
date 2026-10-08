def pytest_addoption(parser):
    parser.addoption(
        "--update-golden",
        action="store_true",
        help="Rewrite golden files instead of comparing. Review the diff before committing.",
    )
