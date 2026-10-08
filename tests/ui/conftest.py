import os

# Run headless unless the caller chose a platform.
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
