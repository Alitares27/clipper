import sys

import gi
gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
from gi.repository import Gdk, GLib, Gtk
from gi.repository import Pango


def request(command):
    print(command, flush=True)
    reply = sys.stdin.readline()
    if not reply:
        app.quit()
    return reply.rstrip("\n")


app = Gtk.Application(application_id="io.github.altairdeb.Clipper")
app.register(None)
if app.get_is_remote():
    app.activate()
    sys.exit(0)

window = Gtk.ApplicationWindow(application=app, title="Clipper · Historial del portapapeles")
window.set_default_size(380, 300)
window.set_resizable(False)
window.set_position(Gtk.WindowPosition.CENTER)
window.set_skip_taskbar_hint(True)
window.set_border_width(10)
window.connect("destroy", lambda *_: app.quit())


def dismiss(_, event):
    if event.keyval == Gdk.KEY_Escape:
        window.hide()
        return True
    return False


window.connect("key-press-event", dismiss)
layout = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
window.add(layout)
search = Gtk.SearchEntry(placeholder_text="Buscar en el historial")
layout.pack_start(search, False, False, 0)
status = Gtk.Label(xalign=0, wrap=True)
layout.pack_end(status, False, False, 0)

cards = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
scroll = Gtk.ScrolledWindow()
scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
scroll.add(cards)
layout.pack_start(scroll, True, True, 0)
last_view = None


def action(_, command, entry_id):
    reply = request(f"{command} {entry_id}")
    if command == "copia" and reply == "OK":
        window.hide()
        GLib.timeout_add(200, paste)
    else:
        status.set_text(reply)
        refresh()


def paste():
    if window.get_visible():
        return False
    reply = request("pega")
    if reply != "OK":
        status.set_text(reply)
    return False


def add_card(entry_id, pinned, preview):
    card = Gtk.Frame()
    row = Gtk.Box(spacing=4, margin=4)
    card.add(row)

    text = Gtk.Button()
    label = Gtk.Label(label=preview, xalign=0, ellipsize=Pango.EllipsizeMode.END)
    label.set_max_width_chars(28)
    text.add(label)
    text.set_tooltip_text(preview)
    text.connect("clicked", action, "copia", entry_id)
    row.pack_start(text, True, True, 0)

    pin = Gtk.Button(label="📌")
    pin_label = "Desfijar" if pinned else "Fijar"
    pin.set_tooltip_text(pin_label)
    pin.get_accessible().set_name(pin_label)
    if pinned:
        pin.get_style_context().add_class("suggested-action")
    pin.connect("clicked", action, "desfija" if pinned else "fija", entry_id)
    row.pack_start(pin, False, False, 0)

    trash = Gtk.Button.new_from_icon_name("user-trash-symbolic", Gtk.IconSize.BUTTON)
    trash.set_tooltip_text("Eliminar")
    trash.get_accessible().set_name("Eliminar")
    trash.connect("clicked", action, "elimina", entry_id)
    row.pack_start(trash, False, False, 0)
    cards.pack_start(card, False, False, 0)


def refresh(*_):
    global last_view
    if not window.get_visible():
        return True
    header = request("lista")
    if not header.isdecimal():
        status.set_text(header)
        return True
    entries = [sys.stdin.readline().rstrip("\n").split("\t", 2) for _ in range(int(header))]
    query = search.get_text().casefold()
    visible = [entry for entry in entries if len(entry) == 3 and query in entry[2].casefold()]
    if visible == last_view:
        return True
    for card in cards.get_children():
        card.destroy()
    for entry in visible:
        add_card(*entry)
    if not visible:
        cards.pack_start(Gtk.Label(label="No hay textos para mostrar."), False, False, 8)
    cards.show_all()
    last_view = visible
    return True


search.connect("search-changed", refresh)


def show_panel(_):
    window.show_all()
    window.present()
    search.grab_focus()
    refresh()


app.connect("activate", show_panel)
GLib.timeout_add(750, refresh)
app.run([])
