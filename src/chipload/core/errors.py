"""Typed errors for core. Avoid broad `except Exception`."""


class ChiploadError(Exception):
    """Base class for all Chipload errors."""


class GeometryError(ChiploadError):
    pass


class DxfImportError(ChiploadError):
    pass


class SelectorError(ChiploadError):
    """A job selector matched nothing, or matched something unexpected."""


class UnsupportedSchemaVersion(ChiploadError):
    """A persisted file has a missing, malformed or newer-than-known schema_version."""

    kind = "File"

    def __init__(self, version: object, current: int) -> None:
        self.version = version
        self.current = current
        super().__init__(
            f"{self.kind} schema_version {version!r} is not supported "
            f"(this app supports up to {current}). Update the application or check the file."
        )


class UnsupportedJobVersion(UnsupportedSchemaVersion):
    kind = "Job"


class UnsupportedPostConfigVersion(UnsupportedSchemaVersion):
    kind = "Post-processor config"


class PostConfigError(ChiploadError):
    """A post-processor config or template cannot be found, parsed or validated."""


class JobFileError(ChiploadError):
    """A job file cannot be read or does not validate."""


class JobError(ChiploadError):
    """A valid job that cannot be machined as written (e.g. depth deeper than the stock)."""


class GcodeCheckFailed(ChiploadError):
    """The independent checker rejected the generated program. No G-code is returned."""

    def __init__(self, errors: list[str]) -> None:
        self.errors = errors
        super().__init__("G-code check failed:\n" + "\n".join(f"  {e}" for e in errors))
