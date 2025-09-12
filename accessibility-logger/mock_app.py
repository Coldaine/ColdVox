#!/usr/bin/env python3
import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk, GLib
import time
import threading

class MockApp:
    def __init__(self):
        self.window = Gtk.Window()
        self.window.set_title("Mock App")
        self.window.connect("destroy", Gtk.main_quit)

        self.textview = Gtk.TextView()
        self.textbuffer = self.textview.get_buffer()
        self.window.add(self.textview)

        self.window.show_all()
        self.window.get_accessible().set_name("MockAppWindow")
        self.textview.get_accessible().set_name("MockAppTextView")

    def run_actions(self):
        # Give the logger time to see the app
        time.sleep(2)

        # Action 1: Set initial text
        GLib.idle_add(self.textbuffer.set_text, "Hello, world!", -1)
        time.sleep(1)

        # Action 2: Insert more text
        GLib.idle_add(self.textbuffer.insert_at_cursor, " This is a test.", -1)
        time.sleep(1)

        # Action 3: Clear the text
        GLib.idle_add(self.textbuffer.set_text, "", -1)
        time.sleep(1)

        # Action 4: Set final text
        GLib.idle_add(self.textbuffer.set_text, "Final content.", -1)
        time.sleep(2)

        # Quit the app
        GLib.idle_add(Gtk.main_quit)

def main():
    app = MockApp()

    # Run the actions in a separate thread
    action_thread = threading.Thread(target=app.run_actions)
    action_thread.daemon = True
    action_thread.start()

    Gtk.main()

if __name__ == "__main__":
    main()
