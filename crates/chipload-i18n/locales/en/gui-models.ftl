# 3D models in the window: File > Add 3D model…, their rows in the Browser and their
# properties (crates/chipload-gui/src/app/models.rs, sources.rs, forms/sources.rs).

models-stl-filter = STL models
models-cannot-read = Cannot read { $path }: { $error }
models-reading = Reading { $file }…
models-cannot-add = Cannot add { $path }:
models-cannot-place = Could not place the model so:
models-removed = Removed model { $id }
dialogs-path-model-title = Add 3D model
dialogs-path-model = An STL file (binary or text):
menu-add-model = Add 3D model…
menu-add-model-hover = An STL file, carved into the board from the top as a relief
menu-add-model-path = Add 3D model from path…
sources-model-title = { $name } (model { $id })
sources-add-model = Add 3D model…
sources-add-model-hover = An STL file, carved into the board from the top as a relief
sources-model-hover = A 3D model: its outline on the board. Double-click for its place, size and depth
sources-model-new-relief = New relief
sources-model-new-relief-hover = Carve this model: a Relief operation within its outline, in the active part
sources-model-properties = Properties
sources-remove-model-hover = Remove the model and its outline; its reliefs stay, each with a problem until it gets another model
forms-model-file = File
forms-model-size = On the board
forms-model-size-hover = Width × height × depth of the model as placed
forms-model-position-hover = The lower-left corner of the model's outline, before it is turned
forms-model-reset-position = Back to the drawing's origin
forms-model-rotation-hover = Turned counter-clockwise about the outline's centre
forms-model-reset-rotation = Not turned
forms-model-width-hover = The model's width on the board; its height follows
forms-model-depth = Depth
forms-model-depth-hover = How deep the relief goes, from the model's top to its lowest point. Reliefs are usually flatter than the model
forms-model-proportional = Keep proportions
forms-model-proportional-hover = The depth follows the width, in the model's own proportion
forms-model-up = Up
forms-model-up-hover = Which of the model's axes points up from the board. Many files are modelled with Y up
forms-model-keep = Keep from top
forms-model-keep-hover = How much of the model is kept, from its top: the rest, its back, is cut away. Width and depth are the part kept.
forms-model-layer-hover = The layer the model's outline is drawn on
forms-model-new-relief = New relief
forms-model-new-relief-hover = Carve this model: a Relief operation within its outline, in the active part
forms-remove-model-hover = Remove the model and its outline; its reliefs stay, each with a problem until it gets another model

# Reliefs and their roughing pass (docs/specs/stl-relief.md §9, §14).
models-could-not-add-roughing = Could not add the roughing pass: { $error }
models-no-roughing-tool = No end mill or plain ball nose in the project or your library
