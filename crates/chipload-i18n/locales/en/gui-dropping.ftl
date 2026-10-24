# Files dragged onto the window: the hint shown while they are over it
# (crates/chipload-gui/src/app/dropping.rs).

dropping-unsupported = Drop a .chipload project, or DXFs and pictures
dropping-open-project = Open this project
dropping-one-project = Drop one .chipload project at a time

# Dropped on an open project.
dropping-add-dxfs = { $dxfs ->
    [one] Add { $dxfs } DXF to the project
   *[other] Add { $dxfs } DXFs to the project
}
dropping-add-pictures = { $pictures ->
    [one] Add { $pictures } picture to the project
   *[other] Add { $pictures } pictures to the project
}
dropping-add-both = Add { $dxfs ->
        [one] { $dxfs } DXF
       *[other] { $dxfs } DXFs
    } and { $pictures ->
        [one] { $pictures } picture
       *[other] { $pictures } pictures
    } to the project

# Dropped when no project is open.
dropping-new-dxfs = { $dxfs ->
    [one] New project with { $dxfs } DXF
   *[other] New project with { $dxfs } DXFs
}
dropping-new-pictures = { $pictures ->
    [one] New project with { $pictures } picture
   *[other] New project with { $pictures } pictures
}
dropping-new-both = New project with { $dxfs ->
        [one] { $dxfs } DXF
       *[other] { $dxfs } DXFs
    } and { $pictures ->
        [one] { $pictures } picture
       *[other] { $pictures } pictures
    }
