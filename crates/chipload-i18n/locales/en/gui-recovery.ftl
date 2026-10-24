# Recovery copies of unsaved work, and the window that offers them at start
# (crates/chipload-gui/src/app/recovery.rs).

recovery-copies-off = Recovery copies are off: { $error }
# Followed by the reason on the next line.
recovery-cannot-open = Cannot open the recovery copy:
recovery-recovered = Recovered unsaved work: { $title }. Save it to keep it.

recovery-title = Recover unsaved work
recovery-intro = Chipload closed without saving these changes:
# A copy of a project that was never saved to a file.
recovery-untitled = Untitled (never saved)

# When the unsaved changes in a copy were made.
recovery-changes-unknown = Changes from at an unknown time
recovery-changes-moments = Changes from less than a minute ago
recovery-changes-minutes = { $count ->
    [one] Changes from { $count } minute ago
   *[other] Changes from { $count } minutes ago
}
recovery-changes-hours = { $count ->
    [one] Changes from { $count } hour ago
   *[other] Changes from { $count } hours ago
}
recovery-changes-days = { $count ->
    [one] Changes from { $count } day ago
   *[other] Changes from { $count } days ago
}

recovery-saved-since = The project file has been saved since this copy was made.
recovery-cannot-be-opened = Can't be opened: { $problem }

# A button: open the copy.
recovery-open = Open
recovery-open-hover = Open it as an unsaved project; save to keep it
recovery-open-dirty = Save or close the current project first
recovery-open-unreadable = This copy can't be read
# A button: decide about the copy another time.
recovery-later = Later
recovery-later-hover = Keep it and ask again next time
