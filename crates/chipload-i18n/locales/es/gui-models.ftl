# 3D models in the window: File > Add 3D model…, their rows in the Browser and their
# properties (crates/chipload-gui/src/app/models.rs, sources.rs, forms/sources.rs).

models-stl-filter = Modelos STL
models-cannot-read = No se puede leer { $path }: { $error }
models-cannot-add = No se puede añadir { $path }:
models-cannot-place = No se pudo colocar el modelo así:
models-removed = Modelo { $id } eliminado
dialogs-path-model-title = Añadir modelo 3D
dialogs-path-model = Un archivo STL (binario o de texto):
menu-add-model = Añadir modelo 3D…
menu-add-model-hover = Un archivo STL, tallado en el tablero desde arriba como relieve
menu-add-model-path = Añadir modelo 3D desde una ruta…
sources-model-title = { $name } (modelo { $id })
sources-add-model = Añadir modelo 3D…
sources-add-model-hover = Un archivo STL, tallado en el tablero desde arriba como relieve
sources-model-hover = Un modelo 3D: su contorno en el tablero. Doble clic para su posición, tamaño y profundidad
sources-model-new-relief = Nuevo relieve
sources-model-new-relief-hover = Tallar este modelo: una operación Relieve dentro de su contorno, en la pieza activa
sources-model-properties = Propiedades
sources-remove-model-hover = Eliminar el modelo y su contorno; sus relieves quedan, cada uno con un problema hasta que tenga otro modelo
forms-model-file = Archivo
forms-model-size = En el tablero
forms-model-size-hover = Ancho × alto × profundidad del modelo colocado
forms-model-position-hover = La esquina inferior izquierda del contorno del modelo, antes de girarlo
forms-model-reset-position = Volver al origen del dibujo
forms-model-rotation-hover = Girado en sentido antihorario alrededor del centro del contorno
forms-model-reset-rotation = Sin girar
forms-model-width-hover = El ancho del modelo en el tablero; el alto se ajusta
forms-model-depth = Profundidad
forms-model-depth-hover = Cuánto baja el relieve, desde lo alto del modelo hasta su punto más bajo. Los relieves suelen ser más planos que el modelo
forms-model-proportional = Mantener proporciones
forms-model-proportional-hover = La profundidad sigue al ancho, en la proporción del modelo
forms-model-up = Arriba
forms-model-up-hover = Qué eje del modelo apunta hacia arriba desde el tablero. Muchos archivos se modelan con Y hacia arriba
forms-model-layer-hover = La capa en la que se dibuja el contorno del modelo
forms-model-new-relief = Nuevo relieve
forms-model-new-relief-hover = Tallar este modelo: una operación Relieve dentro de su contorno, en la pieza activa
forms-remove-model-hover = Eliminar el modelo y su contorno; sus relieves quedan, cada uno con un problema hasta que tenga otro modelo

# Reliefs and their roughing pass (docs/specs/stl-relief.md §9, §14).
models-could-not-add-roughing = No se pudo añadir la pasada de desbaste: { $error }
models-no-roughing-tool = Ninguna fresa plana ni de punta esférica simple en el proyecto o tu biblioteca
