"""Typed errors for core. Avoid broad `except Exception`."""


class gncamError(Exception):
    """Base class for all gncam errors."""


class GeometryError(gncamError):
    pass


class DxfImportError(gncamError):
    pass


class SelectorError(gncamError):
    """A job selector matched nothing, or matched something unexpected."""


class UnsupportedJobVersion(gncamError):
    def __init__(self, version: object, current: int) -> None:
        super().__init__(
            f"Job schema_version {version!r} is not supported (this app supports up to {current}). "
            "Update the application or check the file."
        )
