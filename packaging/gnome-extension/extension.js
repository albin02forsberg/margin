// Margin active window: tells Margin's activity watcher the focused window's class and title on
// the session bus (org.margin.ActiveWindow, method Get), since GNOME Wayland has no other way.
// Nothing leaves the computer; only programs of this user's session can ask.
import Gio from "gi://Gio";
import { Extension } from "resource:///org/gnome/shell/extensions/extension.js";

const IFACE = `<node><interface name="org.margin.ActiveWindow">
  <method name="Get"><arg type="s" direction="out" name="app"/><arg type="s" direction="out" name="title"/></method>
</interface></node>`;

export default class MarginActiveWindow extends Extension {
  enable() {
    this._obj = Gio.DBusExportedObject.wrapJSObject(IFACE, {
      Get() {
        const w = global.display.focus_window;
        return [w?.get_wm_class() ?? "", w?.get_title() ?? ""];
      },
    });
    this._obj.export(Gio.DBus.session, "/org/margin/ActiveWindow");
    this._name = Gio.bus_own_name(Gio.BusType.SESSION, "org.margin.ActiveWindow", Gio.BusNameOwnerFlags.NONE, null, null, null);
  }

  disable() {
    Gio.bus_unown_name(this._name);
    this._obj.unexport();
    this._obj = null;
  }
}
