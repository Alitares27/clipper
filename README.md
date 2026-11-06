# Clipper

Prototipo en Rust de un historial de portapapeles para sesiones Linux con Wayland.
Requiere Rust 1.85 o posterior, wl-clipboard y un compositor con un protocolo
de control del portapapeles compatible con la version instalada de wl-clipboard.

En una terminal de la sesion Wayland, desde este directorio:

```sh
cargo run
```

Copia texto desde otra aplicacion y vuelve a la terminal para ejecutar:

```text
lista
lista palabra
copia 1
elimina 1
pausa
reanuda
vacia
estado
ayuda
salir
```

Usa el identificador mostrado por lista y pega el texto restaurado en la
aplicacion de destino. El historial es temporal, con un maximo de 200 entradas
UTF-8 de 256 KiB cada una. Si wl-paste reconoce una seleccion como sensible,
Clipper la omite. Las vistas previas aparecen en la terminal y pueden quedar
en su historial de desplazamiento.

El comando `estado` muestra si la captura está activa, en pausa o detenida,
y cuántas entradas hay en el historial.

Para guardar el historial cifrado con SQLCipher, instala las cabeceras de
OpenSSL (en Debian, `libssl-dev`) y ejecuta:

```sh
mkdir -m 700 -p "$HOME/.local/share/clipper"
cargo run --locked -- --persistir "$HOME/.local/share/clipper/history.db"
```

Clipper solicita la frase de paso sin mostrarla. Usa la misma frase al volver
a abrir el archivo. En este modo conserva las 25 entradas sin fijar más
recientes. Usa `fija ID` para excluir una entrada de la purga automática y
`desfija ID` para devolverla al historial normal. `vacia` borra también las
entradas fijadas. La captura de texto conserva el límite de 256 KiB por entrada.

Para instalar Clipper para tu usuario en Debian:

```sh
sudo apt install rustc cargo build-essential pkg-config libssl-dev wl-clipboard python3-gi gir1.2-gtk-3.0 libglib2.0-bin
./install.sh
```

Abre Clipper desde el menú de aplicaciones. En KDE puedes asignar Super+V
al comando `$HOME/.local/bin/clipper --mostrar` en los ajustes de atajos
(reemplaza `$HOME` por tu ruta absoluta si el editor no lo expande).
Escape oculta el panel sin detener la captura. El botón de cerrar finaliza
Clipper. Los iconos de cada tarjeta fijan, desfijan o eliminan la entrada.

Para usar la ventana con tu historial cifrado, inicia Clipper desde una
terminal y desbloquéalo antes de usar el atajo:

```sh
"$HOME/.local/bin/clipper" --gui --persistir "$HOME/.local/share/clipper/history-prueba.db"
```

Al seleccionar un texto, el panel se oculta y Clipper intenta pegarlo con
`ydotool`. Si `ydotoold` no está activo y accesible para tu usuario, el
texto queda en el portapapeles para pegarlo con Ctrl+V.

El instalador local genera el binario y la entrada del menú. La distribución
en Flathub requiere un paquete Flatpak con metadatos AppStream y una
integración de pegado que se pueda verificar dentro del sandbox.
