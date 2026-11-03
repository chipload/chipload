# Materials (docs/specs/materials.md).
materials-folder = Materials
materials-folder-hover = The materials you cut: each tool keeps its own values for them, in its cutting data
materials-new = New material
materials-new-name = New material
materials-tool-count = { $count ->
    [one] { $count } tool
   *[other] { $count } tools
}
materials-note-in-project = this project
materials-use-in-project = Use for all operations
materials-use-in-project-hover = Every operation, and those you add later, takes this material and its values for its tool. The stock stays as it is
materials-count = { $count ->
    [one] { $count } material
   *[other] { $count } materials
} in your library.
materials-about = A material is a name to choose for an operation, under its tool, or for a whole setup, in its properties. Its values are in each tool's cutting data (This computer > Tools): fill them in there, or with Save to … under an operation that cut well.
materials-gone = This material is no longer in your library.
materials-entry-about = An operation with this material cuts with its tool's values for it, from the tool's cutting data.
materials-name = Name
materials-values = Tools with values for it
materials-name-taken = a material named "{ $name }" already exists
materials-saved = Saved in { $name }
materials-no-rows = No tool has values for it yet.
materials-open-tool-hover = Open the tool: its cutting data has the values
materials-rows-about = Add values for another tool in that tool's cutting data.
