"""Typed errors for core. Avoid broad `except Exception`."""


class ChiploadError(Exception):
    """Base class for all Chipload errors."""


class GeometryError(ChiploadError):
    pass


class DxfImportError(ChiploadError):
    pass


class SelectorError(ChiploadError):
    """A job selector matched nothing, or matched something unexpected."""


class UnsupportedJobVersion(ChiploadError):
    def __init__(self, version: object, current: int) -> None:
        super().__init__(
            f"Job schema_version {version!r} is not supported (this app supports up to {current}). "
            "Update the application or check the file."
        )
